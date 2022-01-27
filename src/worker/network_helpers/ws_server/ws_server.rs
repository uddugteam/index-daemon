use crate::worker::helper_functions::add_jsonrpc_version;
use crate::worker::network_helpers::ws_server::jsonrpc_messages::{
    JsonRpcErr, JsonRpcId, JsonRpcRequest, JsonRpcResponse,
};
use crate::worker::network_helpers::ws_server::ws_channel_request::WsChannelRequest;
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

    fn send_error(broadcast_recipient: &Tx, id: Option<JsonRpcId>, code: i64, message: String) {
        let response = JsonRpcResponse::Err {
            id,
            result: JsonRpcErr { code, message },
        };
        let mut response = serde_json::to_string(&response).unwrap();
        add_jsonrpc_version(&mut response);

        let response = Message::from(response);
        let _ = broadcast_recipient.unbounded_send(response);
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

    /// What function does:
    /// -- check whether request is `Ok`
    /// -- if request is `Ok` - call `Self::add_new_channel`
    /// -- else - send error response (call `Self::send_error`)
    fn do_response(
        worker: Arc<Mutex<Worker>>,
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        conn_id: String,
        sub_id: Option<JsonRpcId>,
        request: Result<WsChannelRequest, String>,
        ws_answer_timeout_ms: u64,
    ) {
        match request {
            Ok(channel) => {
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
            }
            Err(e) => {
                Self::send_error(
                    &broadcast_recipient,
                    sub_id,
                    JSONRPC_ERROR_INVALID_REQUEST,
                    e,
                );
            }
        }
    }

    /// Function parses request, then calls `Self::do_response` in a separate thread
    fn process_request(
        request: String,
        worker: &Arc<Mutex<Worker>>,
        peer_map: &PeerMap,
        client_addr: SocketAddr,
        conn_id: &str,
        ws_answer_timeout_ms: u64,
        _graceful_shutdown: &Arc<Mutex<bool>>,
    ) {
        let request_string = request.clone();
        let request = Self::parse_jsonrpc_request(&request);

        match request {
            Ok(request) => {
                let sub_id = request.id.clone();
                let request = Self::parse_ws_channel_request(request);

                let peers = peer_map.lock().unwrap();

                if let Some(broadcast_recipient) = peers.get(&client_addr).cloned() {
                    let conn_id_2 = conn_id.to_string();
                    let worker_2 = Arc::clone(worker);

                    let thread_name = format!(
                        "fn: process_request, addr: {}, request: {:?}",
                        client_addr, request
                    );
                    thread::Builder::new()
                        .name(thread_name)
                        .spawn(move || {
                            Self::do_response(
                                worker_2,
                                &client_addr,
                                broadcast_recipient,
                                conn_id_2,
                                sub_id,
                                request,
                                ws_answer_timeout_ms,
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
    /// Function listens for requests and process them (calls `Self::process_request`)
    async fn handle_connection(
        worker: Arc<Mutex<Worker>>,
        peer_map: PeerMap,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        conn_id: String,
        ws_answer_timeout_ms: u64,
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
                    Self::process_request(
                        request.to_string(),
                        &worker,
                        &peer_map,
                        client_addr,
                        &conn_id,
                        ws_answer_timeout_ms,
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
                Arc::clone(&self.graceful_shutdown),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use crate::config_scheme::config_scheme::ConfigScheme;
    use crate::worker::market_helpers::market_channels::MarketChannels;
    use crate::worker::network_helpers::ws_server::ws_client_for_testing::WsClientForTesting;
    use crate::worker::worker::test::check_worker_subscriptions;
    use crate::worker::worker::Worker;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::mpsc::Receiver;
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use std::thread::JoinHandle;
    use std::time;
    use uuid::Uuid;

    fn start_application(ws_addr: &str) -> (Receiver<JoinHandle<()>>, Arc<Mutex<Worker>>) {
        // To prevent DDoS attack on exchanges
        thread::sleep(time::Duration::from_millis(3000));

        let graceful_shutdown = Arc::new(Mutex::new(false));

        let mut config = ConfigScheme::default();
        config.service.ws = true;
        config.service.ws_addr = ws_addr.to_string();
        config.market.channels = vec![MarketChannels::Trades];

        let (tx, rx) = mpsc::channel();
        let worker = Worker::new(tx, graceful_shutdown);
        worker.lock().unwrap().start(config);

        // Give Websocket server time to start
        thread::sleep(time::Duration::from_millis(1000));

        (rx, worker)
    }

    fn make_request(
        sub_id: &str,
        method: &str,
        coins: &[String],
        exchanges: Option<Vec<String>>,
    ) -> String {
        let request = match exchanges {
            Some(exchanges) => json!({
                "id": sub_id,
                "jsonrpc": "2.0",
                "method": method,
                "params": {
                  "coins": coins,
                  "exchanges": exchanges,
                  "frequency_ms": 100u64
                }
            }),
            None => json!({
                "id": sub_id,
                "jsonrpc": "2.0",
                "method": method,
                "params": {
                  "coins": coins,
                  "frequency_ms": 100u64
                }
            }),
        };

        serde_json::to_string(&request).unwrap()
    }

    fn make_unsub_request(method: &str) -> String {
        let request = json!({
            "id": null,
            "jsonrpc": "2.0",
            "method": "unsubscribe",
            "params": {
              "method": method
            }
        });

        serde_json::to_string(&request).unwrap()
    }

    fn ws_connect_and_send_messages(ws_addr: &str, messages: Vec<String>) {
        let config = ConfigScheme::default();
        let uri = "ws://".to_string() + ws_addr;

        // Do not join
        let _ = thread::spawn(|| {
            let ws_client = WsClientForTesting::new(uri, messages, |_info: String| {});
            ws_client.start();
        });

        // Give Websocket server time to process request
        thread::sleep(time::Duration::from_millis(1000));
    }

    #[test]
    #[serial]
    fn test_worker_add_ws_channel() {
        let ws_addr = "127.0.0.1:8001";
        let (_rx, worker) = start_application(ws_addr);

        let method = "coin_average_price".to_string();
        let sub_id = Uuid::new_v4().to_string();
        let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();

        let request = make_request(&sub_id, &method, &coins, None);

        ws_connect_and_send_messages(ws_addr, vec![request]);

        check_worker_subscriptions(&worker, vec![(sub_id, method, coins)]);
    }

    #[test]
    #[serial]
    fn test_worker_resub_ws_channel() {
        // Resubscribe

        let ws_addr = "127.0.0.1:8002";
        let (_rx, worker) = start_application(ws_addr);

        let mut requests = Vec::new();
        let method = "coin_average_price".to_string();

        let sub_id = Uuid::new_v4().to_string();
        let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();
        let request = make_request(&sub_id, &method, &coins, None);
        requests.push(request);

        let sub_id = Uuid::new_v4().to_string();
        let coins = ["BTC".to_string()].to_vec();
        let request = make_request(&sub_id, &method, &coins, None);
        requests.push(request);

        ws_connect_and_send_messages(ws_addr, requests);

        check_worker_subscriptions(&worker, vec![(sub_id, method, coins)]);
    }

    #[test]
    #[serial]
    fn test_worker_unsub_ws_channel() {
        // Unsubscribe

        let ws_addr = "127.0.0.1:8003";
        let (_rx, worker) = start_application(ws_addr);

        let mut requests = Vec::new();

        let method = "coin_average_price".to_string();
        let sub_id = Uuid::new_v4().to_string();
        let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();
        let request = make_request(&sub_id, &method, &coins, None);
        requests.push(request);

        let request = make_unsub_request(&method);
        requests.push(request);

        ws_connect_and_send_messages(ws_addr, requests);

        check_worker_subscriptions(&worker, Vec::new());
    }
}
