use crate::repository::repositories::RepositoryForF64ByTimestampAndPairTuple;
use crate::worker::helper_functions::date_time_from_timestamp_sec;
use crate::worker::network_helpers::ws_server::coin_average_price_historical_snapshot::CoinAveragePriceHistoricalSnapshots;
use crate::worker::network_helpers::ws_server::hepler_functions::ws_send_response;
use crate::worker::network_helpers::ws_server::jsonrpc_messages::{JsonRpcId, JsonRpcRequest};
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_request::WsChannelRequest;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use crate::worker::worker::Worker;
use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
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

pub struct WsServer {
    pub worker: Arc<Mutex<Worker>>,
    pub ws_addr: String,
    pub ws_answer_timeout_ms: u64,
    pub pair_average_price_repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
    pub graceful_shutdown: Arc<Mutex<bool>>,
}

impl WsServer {
    pub fn start(self) {
        let _ = task::block_on(Self::run(self));
    }

    fn parse_jsonrpc_request(request: &str) -> serde_json::Result<JsonRpcRequest> {
        serde_json::from_str(request)
    }

    fn parse_ws_channel_request(request: JsonRpcRequest) -> Result<WsChannelRequest, String> {
        request.try_into()
    }

    fn send_error(
        broadcast_recipient: &Tx,
        id: Option<JsonRpcId>,
        method: WsChannelName,
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

    /// Function adds new channel to worker or removes existing channel from worker (depends on `channel`)
    fn add_new_channel(
        worker: Arc<Mutex<Worker>>,
        broadcast_recipient: Tx,
        conn_id: String,
        channel: WsChannelRequest,
        ws_answer_timeout_ms: u64,
    ) {
        if !matches!(channel, WsChannelRequest::Unsubscribe { .. }) {
            // Subscribe

            let sub_id = channel.get_id();
            let method = channel.get_method();
            let response_sender = WsChannelResponseSender::new(
                broadcast_recipient.clone(),
                channel,
                ws_answer_timeout_ms,
            );

            let add_ws_channel_result = worker
                .lock()
                .unwrap()
                .add_ws_channel(conn_id, response_sender);

            if let Err(errors) = add_ws_channel_result {
                for error in errors {
                    Self::send_error(
                        &broadcast_recipient,
                        sub_id.clone(),
                        method.clone(),
                        JSONRPC_ERROR_INVALID_PARAMS,
                        error,
                    );

                    // To prevent DDoS attack on a client
                    thread::sleep(time::Duration::from_millis(100));
                }
            }
        } else {
            // Unsubscribe

            let method = channel.get_method();

            worker.lock().unwrap().remove_ws_channel(&(conn_id, method));
        }
    }

    /// Prepares response data and sends to recipient
    fn do_response(
        broadcast_recipient: Tx,
        channel: WsChannelRequest,
        pair_average_price_repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
    ) {
        match channel {
            WsChannelRequest::CoinAveragePriceHistorical {
                id,
                coin,
                interval,
                from,
                to,
            } => {
                let pair_tuple = (coin.to_string(), "USD".to_string());
                let from = date_time_from_timestamp_sec(from as i64);
                let to = date_time_from_timestamp_sec(to as i64);

                if let Some(res) = pair_average_price_repository {
                    let res = res.read_range((from, pair_tuple.clone()), (to, pair_tuple));

                    match res {
                        Ok(res) => {
                            let response = WsChannelResponse {
                                id,
                                result: WsChannelResponsePayload::CoinAveragePriceHistorical {
                                    coin,
                                    values: CoinAveragePriceHistoricalSnapshots::with_interval(
                                        res, interval,
                                    ),
                                },
                            };
                            let _ = ws_send_response(&broadcast_recipient, response, None);
                        }
                        Err(e) => error!("Read range error: {}", e),
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    /// What function does:
    /// -- check whether request is `Ok`
    /// -- if request is `Ok` then:
    /// -- -- if request is `request`, call `Self::do_response`
    /// -- -- if request is `channel`, call `Self::add_new_channel`
    /// -- else - send error response (call `Self::send_error`)
    fn process_ws_channel_request(
        worker: Arc<Mutex<Worker>>,
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        conn_id: String,
        sub_id: Option<JsonRpcId>,
        method: WsChannelName,
        request: Result<WsChannelRequest, String>,
        ws_answer_timeout_ms: u64,
        pair_average_price_repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
    ) {
        match request {
            Ok(channel) => {
                if channel.is_channel() {
                    // It's a channel

                    info!(
                        "Client with addr: {} subscribed to: {:?}",
                        client_addr, channel
                    );

                    Self::add_new_channel(
                        worker,
                        broadcast_recipient,
                        conn_id,
                        channel,
                        ws_answer_timeout_ms,
                    );
                } else {
                    // It's not a channel. It's just a request

                    info!("Client with addr: {} requested: {:?}", client_addr, channel);

                    Self::do_response(broadcast_recipient, channel, pair_average_price_repository);
                }
            }
            Err(e) => {
                Self::send_error(
                    &broadcast_recipient,
                    sub_id,
                    method,
                    JSONRPC_ERROR_INVALID_REQUEST,
                    e,
                );
            }
        }
    }

    /// Function parses request, then calls `Self::process_ws_channel_request` in a separate thread
    fn process_jsonrpc_request(
        request: String,
        worker: &Arc<Mutex<Worker>>,
        peer_map: &PeerMap,
        client_addr: SocketAddr,
        conn_id: &str,
        ws_answer_timeout_ms: u64,
        pair_average_price_repository: &mut Option<RepositoryForF64ByTimestampAndPairTuple>,
        _graceful_shutdown: &Arc<Mutex<bool>>,
    ) {
        let request_string = request.clone();
        let request = Self::parse_jsonrpc_request(&request);

        match request {
            Ok(request) => {
                let sub_id = request.id.clone();
                let method = request.method;
                let request = Self::parse_ws_channel_request(request);

                let peers = peer_map.lock().unwrap();

                if let Some(broadcast_recipient) = peers.get(&client_addr).cloned() {
                    let conn_id_2 = conn_id.to_string();
                    let worker_2 = Arc::clone(worker);
                    let pair_average_price_repository_2 = pair_average_price_repository.clone();

                    let thread_name = format!(
                        "fn: process_jsonrpc_request, addr: {}, request: {:?}",
                        client_addr, request
                    );
                    thread::Builder::new()
                        .name(thread_name)
                        .spawn(move || {
                            Self::process_ws_channel_request(
                                worker_2,
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
                } else {
                    // FSR broadcast recipient is not found
                }
            }
            Err(e) => {
                error!(
                    "Parse request error. Client addr: {}, request: {}, error: {:?}",
                    client_addr, request_string, e
                );
            }
        }
    }

    /// Function handles one connection - function is executing until client is disconnected.
    /// Function listens for requests and process them (calls `Self::process_jsonrpc_request`)
    async fn handle_connection(
        worker: Arc<Mutex<Worker>>,
        peer_map: PeerMap,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        conn_id: String,
        ws_answer_timeout_ms: u64,
        mut pair_average_price_repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
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
                        &worker,
                        &peer_map,
                        client_addr,
                        &conn_id,
                        ws_answer_timeout_ms,
                        &mut pair_average_price_repository,
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
                Arc::clone(&self.worker),
                state.clone(),
                stream,
                client_addr,
                conn_id,
                self.ws_answer_timeout_ms,
                self.pair_average_price_repository.clone(),
                Arc::clone(&self.graceful_shutdown),
            ));
        }

        Ok(())
    }
}
