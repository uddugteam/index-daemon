use crate::repository::repositories::WorkerRepositoriesByPairTuple;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, strip_usd};
use crate::worker::network_helpers::ws_server::candles::Candles;
use crate::worker::network_helpers::ws_server::channels::ws_channel_action::WsChannelAction;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::channels::ws_channel_unsubscribe::WsChannelUnsubscribe;
use crate::worker::network_helpers::ws_server::f64_snapshot::F64Snapshots;
use crate::worker::network_helpers::ws_server::hepler_functions::ws_send_response;
use crate::worker::network_helpers::ws_server::jsonrpc_request::{JsonRpcId, JsonRpcRequest};
use crate::worker::network_helpers::ws_server::requests::ws_method_request::WsMethodRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::{
    CoinPrice, WsChannelResponsePayload,
};
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use crate::worker::network_helpers::ws_server::ws_channels_holder::{
    WsChannelsHolder, WsChannelsHolderKey,
};
use crate::worker::network_helpers::ws_server::ws_request::WsRequest;
use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use chrono::{DateTime, Utc};
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
    prelude::*,
};
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, sync::Mutex, thread, time};
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

const JSONRPC_ERROR_INVALID_REQUEST: i64 = -32600;
const JSONRPC_ERROR_INVALID_PARAMS: i64 = -32602;
const JSONRPC_ERROR_INTERNAL_ERROR: i64 = -32603;

pub struct WsServer {
    pub ws_channels_holder: WsChannelsHolder,
    pub ws_addr: String,
    pub ws_answer_timeout_ms: u64,
    pub pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    pub graceful_shutdown: Arc<Mutex<bool>>,
}

impl WsServer {
    pub fn start(self) {
        let _ = task::block_on(Self::run(self));
    }

    fn parse_jsonrpc_request(request: &str) -> serde_json::Result<JsonRpcRequest> {
        serde_json::from_str(request)
    }

    fn parse_ws_request(request: JsonRpcRequest) -> Result<WsRequest, String> {
        request.try_into()
    }

    fn send_error(
        broadcast_recipient: &Tx,
        id: Option<JsonRpcId>,
        method: Option<WsChannelName>,
        code: i64,
        message: String,
    ) {
        let response = WsChannelResponse {
            id,
            result: WsChannelResponsePayload::Err {
                method,
                code,
                message,
            },
        };
        let _ = ws_send_response(broadcast_recipient, response, None);
    }

    fn subscribe_stage_2(
        ws_channels_holder: &mut WsChannelsHolder,
        broadcast_recipient: &Tx,
        conn_id: String,
        request: WsChannelSubscriptionRequest,
        ws_answer_timeout_ms: u64,
        key: WsChannelsHolderKey,
        error_msg: String,
    ) {
        let sub_id = request.get_id();
        let method = request.get_method();

        if ws_channels_holder.contains_key(&key) {
            let response_sender = WsChannelResponseSender::new(
                broadcast_recipient.clone(),
                request,
                ws_answer_timeout_ms,
            );

            ws_channels_holder.add(&key, (conn_id, response_sender));
        } else {
            Self::send_error(
                broadcast_recipient,
                sub_id,
                Some(method),
                JSONRPC_ERROR_INVALID_PARAMS,
                error_msg,
            );
        }
    }

    fn subscribe_stage_1(
        ws_channels_holder: &mut WsChannelsHolder,
        broadcast_recipient: &Tx,
        conn_id: String,
        request: WsChannelSubscriptionRequest,
        ws_answer_timeout_ms: u64,
    ) {
        let (exchanges, error_msg) = match &request {
            WsChannelSubscriptionRequest::WorkerChannels(..) => (
                vec!["worker".to_string()],
                "Parameter value is wrong: coin.",
            ),
            WsChannelSubscriptionRequest::MarketChannels(request) => (
                request.get_exchanges().to_vec(),
                "One of two parameters is wrong: exchange, coin.",
            ),
        };

        let market_value = request.get_method().get_market_value();
        let pairs: Vec<(String, String)> = request
            .get_coins()
            .iter()
            .map(|v| (v.to_string(), "USD".to_string()))
            .collect();

        Self::unsubscribe(ws_channels_holder, conn_id.clone(), request.clone().into());

        for exchange in exchanges {
            for pair in pairs.clone() {
                let key = (exchange.to_string(), market_value, pair);

                Self::subscribe_stage_2(
                    ws_channels_holder,
                    broadcast_recipient,
                    conn_id.clone(),
                    request.clone(),
                    ws_answer_timeout_ms,
                    key,
                    error_msg.to_string(),
                );

                // To prevent DDoS attack on a client
                thread::sleep(time::Duration::from_millis(100));
            }
        }
    }

    fn unsubscribe(
        ws_channels_holder: &mut WsChannelsHolder,
        conn_id: String,
        request: WsChannelUnsubscribe,
    ) {
        let method = request.method;
        let key = (conn_id, method);

        ws_channels_holder.remove(&key);
    }

    /// Function adds new channel or removes existing channel (depends on `action`)
    fn process_channel_action_request(
        mut ws_channels_holder: WsChannelsHolder,
        broadcast_recipient: Tx,
        conn_id: String,
        action: WsChannelAction,
        ws_answer_timeout_ms: u64,
    ) {
        match action {
            WsChannelAction::Subscribe(request) => {
                Self::subscribe_stage_1(
                    &mut ws_channels_holder,
                    &broadcast_recipient,
                    conn_id,
                    request,
                    ws_answer_timeout_ms,
                );
            }
            WsChannelAction::Unsubscribe(request) => {
                Self::unsubscribe(&mut ws_channels_holder, conn_id, request);
            }
        }
    }

    fn one_hour_before(timestamp: DateTime<Utc>) -> DateTime<Utc> {
        let one_hour_in_seconds = 3600;
        let timestamp = timestamp.timestamp() - one_hour_in_seconds;

        date_time_from_timestamp_sec(timestamp as u64)
    }

    /// Prepares response data and sends to recipient
    fn do_response(
        broadcast_recipient: Tx,
        sub_id: Option<JsonRpcId>,
        request: WsMethodRequest,
        pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    ) {
        match request.clone() {
            WsMethodRequest::AvailableCoins { id } => {
                let to = Utc::now();
                let from = Self::one_hour_before(to);

                if let Some(repositories) = pair_average_price_repositories {
                    let mut res = Ok(Vec::new());

                    for (pair_tuple, repository) in repositories {
                        if let Some(coin) = strip_usd(&pair_tuple) {
                            match repository.read_range(from, to) {
                                Ok(values) => {
                                    let mut values: Vec<(DateTime<Utc>, f64)> =
                                        values.into_iter().map(|(k, v)| (k, v)).collect();
                                    values.sort_by(|a, b| a.0.cmp(&b.0));

                                    let value = *values.last().unwrap();
                                    let value = CoinPrice {
                                        coin,
                                        value: value.1,
                                    };

                                    if let Ok(v) = res.as_mut() {
                                        v.push(value);
                                    }
                                }
                                Err(e) => {
                                    error!("Available coins. Read range error: {}", e);

                                    res =
                                        Err("Internal error. Read available coins error."
                                            .to_string());
                                    break;
                                }
                            }
                        }
                    }

                    match res {
                        Ok(coins) => {
                            let response = WsChannelResponse {
                                id,
                                result: WsChannelResponsePayload::AvailableCoins {
                                    coins,
                                    timestamp: to,
                                },
                            };
                            let _ = ws_send_response(
                                &broadcast_recipient,
                                response,
                                Some(request.get_method()),
                            );
                        }
                        Err(e) => {
                            Self::send_error(
                                &broadcast_recipient,
                                sub_id,
                                Some(request.get_method()),
                                JSONRPC_ERROR_INTERNAL_ERROR,
                                e,
                            );
                        }
                    }
                }
            }
            WsMethodRequest::CoinAveragePriceHistorical {
                id,
                coin,
                interval,
                from,
                to,
            }
            | WsMethodRequest::CoinAveragePriceCandlesHistorical {
                id,
                coin,
                interval,
                from,
                to,
            } => {
                let pair_tuple = (coin.to_string(), "USD".to_string());
                let from = date_time_from_timestamp_sec(from);
                let to = to
                    .map(date_time_from_timestamp_sec)
                    .unwrap_or_else(Utc::now);

                if let Some(repositories) = pair_average_price_repositories {
                    if let Some(repository) = repositories.get(&pair_tuple) {
                        match repository.read_range(from, to) {
                            Ok(values) => {
                                let values = values.into_iter().map(|(k, v)| (k, v)).collect();

                                let response = match request {
                                    WsMethodRequest::CoinAveragePriceHistorical { .. } => WsChannelResponse {
                                        id,
                                        result: WsChannelResponsePayload::CoinAveragePriceHistorical {
                                            coin,
                                            values: F64Snapshots::with_interval(
                                                values, interval,
                                            ),
                                        },
                                    },
                                    WsMethodRequest::CoinAveragePriceCandlesHistorical { .. } => WsChannelResponse {
                                        id,
                                        result: WsChannelResponsePayload::CoinAveragePriceCandlesHistorical {
                                            coin,
                                            values: Candles::calculate(values, interval),
                                        },
                                    },
                                    _ => unreachable!(),
                                };
                                let _ = ws_send_response(&broadcast_recipient, response, None);
                            }
                            Err(e) => {
                                error!("Historical. Read range error: {}", e);

                                Self::send_error(
                                    &broadcast_recipient,
                                    sub_id,
                                    Some(request.get_method()),
                                    JSONRPC_ERROR_INTERNAL_ERROR,
                                    "Internal error. Read historical data error.".to_string(),
                                );
                            }
                        }
                    } else {
                        Self::send_error(
                            &broadcast_recipient,
                            sub_id,
                            Some(request.get_method()),
                            JSONRPC_ERROR_INVALID_PARAMS,
                            format!("Coin {} not supported.", coin),
                        );
                    }
                }
            }
        }
    }

    /// What function does:
    /// -- check whether request is `Ok`
    /// -- if request is `Ok` then:
    /// -- -- if request is `request`, call `Self::do_response`
    /// -- -- if request is `channel`, call `Self::process_channel_action_request`
    /// -- else - send error response (call `Self::send_error`)
    fn process_ws_channel_request(
        ws_channels_holder: WsChannelsHolder,
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        conn_id: String,
        sub_id: Option<JsonRpcId>,
        method: WsChannelName,
        request: Result<WsRequest, String>,
        ws_answer_timeout_ms: u64,
        pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    ) {
        match request {
            Ok(request) => match request {
                WsRequest::Channel(request) => {
                    info!(
                        "Client with addr: {} subscribed to: {:?}",
                        client_addr, request
                    );

                    Self::process_channel_action_request(
                        ws_channels_holder,
                        broadcast_recipient,
                        conn_id,
                        request,
                        ws_answer_timeout_ms,
                    );
                }
                WsRequest::Method(request) => {
                    info!("Client with addr: {} requested: {:?}", client_addr, request);

                    Self::do_response(
                        broadcast_recipient,
                        sub_id,
                        request,
                        pair_average_price_repositories,
                    );
                }
            },
            Err(e) => {
                Self::send_error(
                    &broadcast_recipient,
                    sub_id,
                    Some(method),
                    JSONRPC_ERROR_INVALID_REQUEST,
                    e,
                );
            }
        }
    }

    /// Function parses request, then calls `Self::process_ws_channel_request` in a separate thread
    fn process_jsonrpc_request(
        request: String,
        ws_channels_holder: &WsChannelsHolder,
        peer_map: &PeerMap,
        client_addr: SocketAddr,
        conn_id: &str,
        ws_answer_timeout_ms: u64,
        pair_average_price_repositories: &mut Option<WorkerRepositoriesByPairTuple>,
        _graceful_shutdown: &Arc<Mutex<bool>>,
    ) {
        let peers = peer_map.lock().unwrap();

        if let Some(broadcast_recipient) = peers.get(&client_addr).cloned() {
            match Self::parse_jsonrpc_request(&request) {
                Ok(request) => {
                    let sub_id = request.id.clone();
                    let method = request.method;
                    let request = Self::parse_ws_request(request);

                    let conn_id_2 = conn_id.to_string();
                    let ws_channels_holder_2 = ws_channels_holder.clone();
                    let pair_average_price_repository_2 = pair_average_price_repositories.clone();

                    let thread_name = format!(
                        "fn: process_jsonrpc_request, addr: {}, request: {:?}",
                        client_addr, request
                    );
                    thread::Builder::new()
                        .name(thread_name)
                        .spawn(move || {
                            Self::process_ws_channel_request(
                                ws_channels_holder_2,
                                &client_addr,
                                broadcast_recipient,
                                conn_id_2,
                                sub_id,
                                method,
                                request,
                                ws_answer_timeout_ms,
                                pair_average_price_repository_2,
                            )
                        })
                        .unwrap();
                }
                Err(e) => {
                    Self::send_error(
                        &broadcast_recipient,
                        None,
                        None,
                        JSONRPC_ERROR_INVALID_REQUEST,
                        e.to_string(),
                    );
                }
            }
        } else {
            // FSR broadcast recipient is not found
        }
    }

    /// Function handles one connection - function is executing until client is disconnected.
    /// Function listens for requests and process them (calls `Self::process_jsonrpc_request`)
    async fn handle_connection(
        ws_channels_holder: WsChannelsHolder,
        peer_map: PeerMap,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        conn_id: String,
        ws_answer_timeout_ms: u64,
        mut pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
        graceful_shutdown: Arc<Mutex<bool>>,
    ) {
        match async_tungstenite::accept_async(raw_stream).await {
            Ok(ws_stream) => {
                info!(
                    "WebSocket connection established, client addr: {}.",
                    client_addr
                );

                // Insert the write part of this peer to the peer map.
                let (tx, rx) = unbounded();
                peer_map.lock().unwrap().insert(client_addr, tx);

                let (outgoing, incoming) = ws_stream.split();

                // TODO: Replace for_each with a simple loop (this is needed for graceful_shutdown)
                let broadcast_incoming = incoming.try_for_each(|request| {
                    Self::process_jsonrpc_request(
                        request.to_string(),
                        &ws_channels_holder,
                        &peer_map,
                        client_addr,
                        &conn_id,
                        ws_answer_timeout_ms,
                        &mut pair_average_price_repositories,
                        &graceful_shutdown,
                    );
                    future::ok(())
                });

                let receive_from_others = rx.map(Ok).forward(outgoing);

                pin_mut!(broadcast_incoming, receive_from_others);
                future::select(broadcast_incoming, receive_from_others).await;

                // The client is already disconnected on this line
                peer_map.lock().unwrap().remove(&client_addr);
            }
            Err(e) => {
                error!(
                    "Error during the websocket handshake occurred. Error: {:?}",
                    e
                );
            }
        }
    }

    /// Function listens and establishes connections. Function never ends.
    async fn run(self) -> Result<(), io::Error> {
        let state = PeerMap::new(Mutex::new(HashMap::new()));

        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = TcpListener::bind(&self.ws_addr).await;
        let listener = try_socket.expect("Failed to bind");
        info!("Websocket server started on: {}", self.ws_addr);

        // Let's spawn the handling of each connection in a separate task.
        while let Ok((stream, client_addr)) = listener.accept().await {
            if *self.graceful_shutdown.lock().unwrap() {
                break;
            }

            let conn_id = Uuid::new_v4().to_string();

            let _ = task::spawn(Self::handle_connection(
                self.ws_channels_holder.clone(),
                state.clone(),
                stream,
                client_addr,
                conn_id,
                self.ws_answer_timeout_ms,
                self.pair_average_price_repositories.clone(),
                Arc::clone(&self.graceful_shutdown),
            ));
        }

        Ok(())
    }
}
