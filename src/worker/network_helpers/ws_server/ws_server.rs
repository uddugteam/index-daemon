use crate::graceful_shutdown::GracefulShutdown;
use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::helper_functions::strip_usd;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use crate::worker::market_helpers::pair_average_price::StoredAndWsTransmissibleF64ByPairTuple;
use crate::worker::market_helpers::percent_change::PercentChange;
use crate::worker::network_helpers::ws_server::candles::Candles;
use crate::worker::network_helpers::ws_server::channels::market_channels::LocalMarketChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_action::WsChannelAction;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::connection_id::ConnectionId;
use crate::worker::network_helpers::ws_server::f64_snapshot::F64Snapshots;
use crate::worker::network_helpers::ws_server::hepler_functions::ws_send_response;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderKey;
use crate::worker::network_helpers::ws_server::holders::percent_change_holder::PercentChangeByIntervalHolder;
use crate::worker::network_helpers::ws_server::holders::ws_channels_holder::WsChannelsHolder;
use crate::worker::network_helpers::ws_server::jsonrpc_request::{JsonRpcId, JsonRpcRequest};
use crate::worker::network_helpers::ws_server::requests::ws_method_request::WsMethodRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::{
    CoinPrice, WsChannelResponsePayload,
};
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use crate::worker::network_helpers::ws_server::ws_request::WsRequest;
use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use chrono::Utc;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
    prelude::*,
};
use std::net::SocketAddr;

type Tx = UnboundedSender<Message>;

const JSONRPC_ERROR_INVALID_REQUEST: i64 = -32600;
const JSONRPC_ERROR_INVALID_PARAMS: i64 = -32602;
const JSONRPC_ERROR_INTERNAL_ERROR: i64 = -32603;

/// We need this workaround because we can't pass repository into `fn subscribe_stage_1()`
#[derive(Clone)]
struct RepositoryWrap(pub Option<RepositoryForF64ByTimestamp>);

pub struct WsServer {
    pub percent_change_holder: PercentChangeByIntervalHolder,
    pub ws_channels_holder: WsChannelsHolder,
    pub ws_addr: String,
    pub ws_answer_timeout_ms: u64,
    pub percent_change_interval_sec: u64,
    pub index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pub pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    pub graceful_shutdown: GracefulShutdown,
}

impl WsServer {
    fn parse_jsonrpc_request(request: &str) -> serde_json::Result<JsonRpcRequest> {
        serde_json::from_str(request)
    }

    fn parse_ws_request(
        request: JsonRpcRequest,
        ws_answer_timeout_ms: u64,
        percent_change_interval_sec: u64,
    ) -> Result<WsRequest, String> {
        WsRequest::new(request, ws_answer_timeout_ms, percent_change_interval_sec)
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
            result: WsChannelResponsePayload::Err { code, message },
        };
        let _ = ws_send_response(broadcast_recipient, response, method);
    }

    async fn subscribe_stage_2(
        percent_change_holder: &mut PercentChangeByIntervalHolder,
        ws_channels_holder: &mut WsChannelsHolder,
        request: WsChannelSubscriptionRequest,
        holder_key: HolderKey,
        sender: WsChannelResponseSender,
        subscriber: CJ,
        repository: RepositoryWrap,
    ) {
        let conn_id = subscriber.0.clone();
        let percent_change_interval_sec = request.get_percent_change_interval_sec();

        if let Some(percent_change_interval_sec) = percent_change_interval_sec {
            if let Some(interval_exists) = percent_change_holder
                .contains_interval(&holder_key, percent_change_interval_sec)
                .await
            {
                if interval_exists {
                    // add subscriber

                    percent_change_holder
                        .add_subscriber(&holder_key, percent_change_interval_sec, subscriber)
                        .await;
                } else {
                    // add interval

                    let percent_change = PercentChange::from_historical(
                        repository.0,
                        percent_change_interval_sec,
                        subscriber,
                    )
                    .await;

                    percent_change_holder
                        .add_interval(&holder_key, percent_change_interval_sec, percent_change)
                        .await;
                }
            } else {
                unreachable!(
                    "Internal error: fn {}, \"holder_key\": {:?} is not present",
                    "subscribe_stage_2", holder_key,
                );
            }
        }

        ws_channels_holder.add(&holder_key, conn_id, sender).await;
    }

    fn make_holder_keys(
        percent_change_holder: &PercentChangeByIntervalHolder,
        ws_channels_holder: &WsChannelsHolder,
        exchanges: Vec<MarketValueOwner>,
        market_value: MarketValue,
        pairs: Vec<Option<(String, String)>>,
    ) -> Option<Vec<HolderKey>> {
        let mut keys = Vec::new();

        for exchange in &exchanges {
            for pair in pairs.clone() {
                let key = (exchange.clone(), market_value, pair);

                if percent_change_holder.contains_holder_key(&key)
                    && ws_channels_holder.contains_key(&key)
                {
                    // Good key

                    keys.push(key);
                } else {
                    // Inexistent key

                    return None;
                }
            }
        }

        Some(keys)
    }

    async fn get_repository_for_request(
        holder_key: &HolderKey,
        index_price_repository: &RepositoryWrap,
        pair_average_price: &StoredAndWsTransmissibleF64ByPairTuple,
    ) -> RepositoryWrap {
        match holder_key.0 {
            MarketValueOwner::Worker => match holder_key.1 {
                MarketValue::IndexPrice => index_price_repository.clone(),
                MarketValue::PairAveragePrice => match &holder_key.2 {
                    Some(pair) => {
                        let repository = if let Some(arc) = pair_average_price.get(pair) {
                            arc.read().await.repository.clone()
                        } else {
                            unreachable!("Internal error: fn {}, \"pair_average_price\" does not contain the pair {:?}", "get_repository_for_request", pair);
                        };

                        RepositoryWrap(repository)
                    }
                    None => unreachable!(),
                },
                MarketValue::PairExchangePrice | MarketValue::PairExchangeVolume => {
                    unreachable!();
                }
            },
            MarketValueOwner::Market(_) => unreachable!(),
        }
    }

    async fn subscribe_stage_1(
        percent_change_holder: &mut PercentChangeByIntervalHolder,
        ws_channels_holder: &mut WsChannelsHolder,
        broadcast_recipient: &Tx,
        conn_id: ConnectionId,
        request: WsChannelSubscriptionRequest,
        index_price_repository: RepositoryWrap,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    ) {
        let sub_id = request.get_id();

        let (exchanges, error_msg) = match &request {
            WsChannelSubscriptionRequest::Worker(..) => (
                vec![MarketValueOwner::Worker],
                "Parameter value is wrong: coin.",
            ),
            WsChannelSubscriptionRequest::Market(request) => {
                let exchanges = match request {
                    LocalMarketChannels::CoinExchangePrice { exchanges, .. }
                    | LocalMarketChannels::CoinExchangeVolume { exchanges, .. } => exchanges
                        .iter()
                        .cloned()
                        .map(MarketValueOwner::Market)
                        .collect(),
                };

                (exchanges, "One of two parameters is wrong: exchange, coin.")
            }
        };

        let market_value = request.get_method().get_market_value();
        let pairs: Vec<Option<(String, String)>> = match request.get_coins() {
            Some(coins) => coins
                .iter()
                .map(|v| (v.to_string(), "USD".to_string()))
                .map(Some)
                .collect(),
            None => vec![None],
        };

        let holder_keys = Self::make_holder_keys(
            percent_change_holder,
            ws_channels_holder,
            exchanges,
            market_value,
            pairs,
        );

        if let Some(holder_keys) = holder_keys {
            let subscriber = (conn_id, sub_id);

            Self::unsubscribe(
                percent_change_holder,
                ws_channels_holder,
                subscriber.clone(),
            )
            .await;

            let sender = WsChannelResponseSender::new(broadcast_recipient.clone(), request.clone());

            for holder_key in holder_keys {
                let repository = Self::get_repository_for_request(
                    &holder_key,
                    &index_price_repository,
                    &pair_average_price,
                )
                .await;

                Self::subscribe_stage_2(
                    percent_change_holder,
                    ws_channels_holder,
                    request.clone(),
                    holder_key,
                    sender.clone(),
                    subscriber.clone(),
                    repository,
                )
                .await;
            }

            let _ = sender.send_succ_sub_notif();
        } else {
            let method = request.get_method();

            Self::send_error(
                broadcast_recipient,
                Some(sub_id),
                Some(method),
                JSONRPC_ERROR_INVALID_PARAMS,
                error_msg.to_string(),
            );
        }
    }

    async fn unsubscribe(
        percent_change_holder: &mut PercentChangeByIntervalHolder,
        ws_channels_holder: &mut WsChannelsHolder,
        key: CJ,
    ) {
        percent_change_holder.remove(&key).await;
        ws_channels_holder.remove(&key).await;
    }

    /// Function adds new channel or removes existing channel (depends on `action`)
    async fn process_channel_action_request(
        mut percent_change_holder: PercentChangeByIntervalHolder,
        mut ws_channels_holder: WsChannelsHolder,
        broadcast_recipient: Tx,
        conn_id: ConnectionId,
        action: WsChannelAction,
        index_price_repository: Option<RepositoryForF64ByTimestamp>,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    ) {
        match action {
            WsChannelAction::Subscribe(request) => {
                Self::subscribe_stage_1(
                    &mut percent_change_holder,
                    &mut ws_channels_holder,
                    &broadcast_recipient,
                    conn_id,
                    request,
                    RepositoryWrap(index_price_repository),
                    pair_average_price,
                )
                .await;
            }
            WsChannelAction::Unsubscribe(request) => {
                Self::unsubscribe(
                    &mut percent_change_holder,
                    &mut ws_channels_holder,
                    (conn_id, request.id),
                )
                .await;
            }
        }
    }

    async fn response_1(
        broadcast_recipient: Tx,
        sub_id: JsonRpcId,
        request: WsMethodRequest,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    ) -> Option<()> {
        match request.clone() {
            WsMethodRequest::AvailableCoins { id } => {
                let mut res = Ok(Vec::new());

                for (pair_tuple, repository) in pair_average_price {
                    if let Some(coin) = strip_usd(&pair_tuple) {
                        if let Some(value) = repository.read().await.get() {
                            let value = CoinPrice { coin, value };

                            if let Ok(v) = res.as_mut() {
                                v.push(value);
                            }
                        }
                    }
                }

                match res {
                    Ok(coins) => {
                        let response = WsChannelResponse {
                            id: Some(id),
                            result: WsChannelResponsePayload::AvailableCoins {
                                coins,
                                timestamp: Utc::now(),
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
                            Some(sub_id),
                            Some(request.get_method()),
                            JSONRPC_ERROR_INTERNAL_ERROR,
                            e,
                        );
                    }
                }
            }
            _ => unreachable!(),
        }

        Some(())
    }

    async fn response_2(
        broadcast_recipient: Tx,
        sub_id: JsonRpcId,
        request: WsMethodRequest,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    ) -> Option<()> {
        match request.clone() {
            WsMethodRequest::CoinAveragePriceHistorical {
                id,
                coin,
                interval_sec,
                from,
                to,
            }
            | WsMethodRequest::CoinAveragePriceCandlesHistorical {
                id,
                coin,
                interval_sec,
                from,
                to,
            } => {
                let pair_tuple = (coin.to_string(), "USD".to_string());

                if let Some(repository) = pair_average_price.get(&pair_tuple) {
                    if let Some(repository) = repository.read().await.repository.clone() {
                        match repository.read_range(from..to).await {
                            Ok(values) => {
                                let response = match request {
                                    WsMethodRequest::CoinAveragePriceHistorical { .. } => WsChannelResponse {
                                        id:Some(id),
                                        result: WsChannelResponsePayload::CoinAveragePriceHistorical {
                                            coin,
                                            values: F64Snapshots::with_interval(
                                                values, interval_sec,
                                            ),
                                        },
                                    },
                                    WsMethodRequest::CoinAveragePriceCandlesHistorical { .. } => WsChannelResponse {
                                        id:Some(id),
                                        result: WsChannelResponsePayload::CoinAveragePriceCandlesHistorical {
                                            coin,
                                            values: Candles::calculate(values, interval_sec),
                                        },
                                    },
                                    _ => unreachable!(),
                                };
                                let _ = ws_send_response(
                                    &broadcast_recipient,
                                    response,
                                    Some(request.get_method()),
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Method: {:?}. Read range error: {}",
                                    request.get_method(),
                                    e
                                );

                                Self::send_error(
                                    &broadcast_recipient,
                                    Some(sub_id),
                                    Some(request.get_method()),
                                    JSONRPC_ERROR_INTERNAL_ERROR,
                                    "Internal error. Read historical data error.".to_string(),
                                );
                            }
                        }
                    }
                } else {
                    Self::send_error(
                        &broadcast_recipient,
                        Some(sub_id),
                        Some(request.get_method()),
                        JSONRPC_ERROR_INVALID_PARAMS,
                        format!("Coin {} not supported.", coin),
                    );
                }
            }
            _ => unreachable!(),
        }

        Some(())
    }

    async fn response_3(
        broadcast_recipient: Tx,
        sub_id: JsonRpcId,
        request: WsMethodRequest,
        index_price_repository: Option<RepositoryForF64ByTimestamp>,
    ) -> Option<()> {
        match request.clone() {
            WsMethodRequest::IndexPriceHistorical {
                id,
                interval_sec,
                from,
                to,
            }
            | WsMethodRequest::IndexPriceCandlesHistorical {
                id,
                interval_sec,
                from,
                to,
            } => {
                let repository = index_price_repository?;

                match repository.read_range(from..to).await {
                    Ok(values) => {
                        let response = match request {
                            WsMethodRequest::IndexPriceHistorical { .. } => WsChannelResponse {
                                id: Some(id),
                                result: WsChannelResponsePayload::IndexPriceHistorical {
                                    values: F64Snapshots::with_interval(values, interval_sec),
                                },
                            },
                            WsMethodRequest::IndexPriceCandlesHistorical { .. } => {
                                WsChannelResponse {
                                    id: Some(id),
                                    result: WsChannelResponsePayload::IndexPriceCandlesHistorical {
                                        values: Candles::calculate(values, interval_sec),
                                    },
                                }
                            }
                            _ => unreachable!(),
                        };
                        let _ = ws_send_response(
                            &broadcast_recipient,
                            response,
                            Some(request.get_method()),
                        );
                    }
                    Err(e) => {
                        error!(
                            "Method: {:?}. Read range error: {}",
                            request.get_method(),
                            e
                        );

                        Self::send_error(
                            &broadcast_recipient,
                            Some(sub_id),
                            Some(request.get_method()),
                            JSONRPC_ERROR_INTERNAL_ERROR,
                            "Internal error. Read historical data error.".to_string(),
                        );
                    }
                }
            }
            _ => unreachable!(),
        }

        Some(())
    }

    /// Prepares response data and sends to recipient
    async fn do_response(
        broadcast_recipient: Tx,
        sub_id: JsonRpcId,
        request: WsMethodRequest,
        index_price_repository: Option<RepositoryForF64ByTimestamp>,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    ) {
        match request.clone() {
            WsMethodRequest::AvailableCoins { .. } => {
                Self::response_1(broadcast_recipient, sub_id, request, pair_average_price).await;
            }
            WsMethodRequest::IndexPriceHistorical { .. }
            | WsMethodRequest::IndexPriceCandlesHistorical { .. } => {
                Self::response_3(broadcast_recipient, sub_id, request, index_price_repository)
                    .await;
            }
            WsMethodRequest::CoinAveragePriceHistorical { .. }
            | WsMethodRequest::CoinAveragePriceCandlesHistorical { .. } => {
                Self::response_2(broadcast_recipient, sub_id, request, pair_average_price).await;
            }
        }
    }

    /// What function does:
    /// -- check whether request is `Ok`
    /// -- if request is `Ok` then:
    /// -- -- if request is `request`, call `Self::do_response`
    /// -- -- if request is `channel`, call `Self::process_channel_action_request`
    /// -- else - send error response (call `Self::send_error`)
    async fn process_ws_channel_request(
        percent_change_holder: PercentChangeByIntervalHolder,
        ws_channels_holder: WsChannelsHolder,
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        conn_id: ConnectionId,
        sub_id: JsonRpcId,
        method: WsChannelName,
        request: Result<WsRequest, String>,
        index_price_repository: Option<RepositoryForF64ByTimestamp>,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    ) {
        match request {
            Ok(request) => match request {
                WsRequest::Channel(request) => {
                    debug!(
                        "Client with addr: {} subscribed to: {:?}",
                        client_addr, request
                    );

                    Self::process_channel_action_request(
                        percent_change_holder,
                        ws_channels_holder,
                        broadcast_recipient,
                        conn_id,
                        request,
                        index_price_repository,
                        pair_average_price,
                    )
                    .await;
                }
                WsRequest::Method(request) => {
                    debug!("Client with addr: {} requested: {:?}", client_addr, request);

                    Self::do_response(
                        broadcast_recipient,
                        sub_id,
                        request,
                        index_price_repository,
                        pair_average_price,
                    )
                    .await;
                }
            },
            Err(e) => {
                Self::send_error(
                    &broadcast_recipient,
                    Some(sub_id),
                    Some(method),
                    JSONRPC_ERROR_INVALID_REQUEST,
                    e,
                );
            }
        }
    }

    /// Function parses request, then calls `Self::process_ws_channel_request` in a separate thread
    async fn process_jsonrpc_request(
        request: String,
        percent_change_holder: &PercentChangeByIntervalHolder,
        ws_channels_holder: &WsChannelsHolder,
        broadcast_recipient: Tx,
        client_addr: SocketAddr,
        conn_id: &ConnectionId,
        ws_answer_timeout_ms: u64,
        percent_change_interval_sec: u64,
        index_price_repository: Option<RepositoryForF64ByTimestamp>,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
        _graceful_shutdown: &GracefulShutdown,
    ) -> Result<(), async_tungstenite::tungstenite::Error> {
        match Self::parse_jsonrpc_request(&request) {
            Ok(request) => {
                let sub_id = request.id.clone();
                let method = request.method;
                let request = Self::parse_ws_request(
                    request,
                    ws_answer_timeout_ms,
                    percent_change_interval_sec,
                );

                let conn_id_2 = conn_id.clone();
                let percent_change_holder_2 = percent_change_holder.clone();
                let ws_channels_holder_2 = ws_channels_holder.clone();
                let index_price_repository_2 = index_price_repository.clone();
                let pair_average_price_2 = pair_average_price.clone();

                Self::process_ws_channel_request(
                    percent_change_holder_2,
                    ws_channels_holder_2,
                    &client_addr,
                    broadcast_recipient,
                    conn_id_2,
                    sub_id,
                    method,
                    request,
                    index_price_repository_2,
                    pair_average_price_2,
                )
                .await;
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

        Ok(())
    }

    /// Function handles one connection - function is executing until client is disconnected.
    /// Function listens for requests and process them (calls `Self::process_jsonrpc_request`)
    async fn handle_connection(
        percent_change_holder: PercentChangeByIntervalHolder,
        ws_channels_holder: WsChannelsHolder,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        conn_id: ConnectionId,
        ws_answer_timeout_ms: u64,
        percent_change_interval_sec: u64,
        index_price_repository: Option<RepositoryForF64ByTimestamp>,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
        graceful_shutdown: GracefulShutdown,
    ) {
        match async_tungstenite::accept_async(raw_stream).await {
            Ok(ws_stream) => {
                debug!(
                    "WebSocket connection established, client addr: {}.",
                    client_addr
                );

                // Insert the write part of this peer to the peer map.
                let (broadcast_recipient, rx) = unbounded();

                let (outgoing, incoming) = ws_stream.split();

                // TODO: Replace for_each with a simple loop (this is needed for graceful_shutdown)
                let broadcast_incoming = incoming.try_for_each(|request| {
                    Self::process_jsonrpc_request(
                        request.to_string(),
                        &percent_change_holder,
                        &ws_channels_holder,
                        broadcast_recipient.clone(),
                        client_addr,
                        &conn_id,
                        ws_answer_timeout_ms,
                        percent_change_interval_sec,
                        index_price_repository.clone(),
                        pair_average_price.clone(),
                        &graceful_shutdown,
                    )
                });

                let receive_from_others = rx.map(Ok).forward(outgoing);

                pin_mut!(broadcast_incoming, receive_from_others);
                future::select(broadcast_incoming, receive_from_others).await;
            }
            Err(e) => {
                error!(
                    "Error during the websocket handshake occurred. Client addr: {:?}, Error: {:?}",
                    client_addr, e,
                );
            }
        }
    }

    /// Function listens and establishes connections. Function never ends.
    pub async fn run(self) {
        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = TcpListener::bind(&self.ws_addr).await;
        let listener = try_socket.expect("Failed to bind");
        debug!("Websocket server started on: {}", self.ws_addr);

        // Let's spawn the handling of each connection in a separate task.
        while let Ok((stream, client_addr)) = listener.accept().await {
            if self.graceful_shutdown.get().await {
                break;
            }

            let conn_id = ConnectionId::new();

            let _ = task::spawn(Self::handle_connection(
                self.percent_change_holder.clone(),
                self.ws_channels_holder.clone(),
                stream,
                client_addr,
                conn_id,
                self.ws_answer_timeout_ms,
                self.percent_change_interval_sec,
                self.index_price_repository.clone(),
                self.pair_average_price.clone(),
                self.graceful_shutdown.clone(),
            ));
        }
    }
}
