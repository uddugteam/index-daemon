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
use jsonrpc_core::{IoHandler, Params, Value};
use rustc_serialize::json::Json;
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, sync::Mutex, thread};
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

// pub const JSONRPC_REQUEST: &str = r#"{"id": null, "jsonrpc": "2.0", "method": "my_method", "params": {}}"#;
pub const JSONRPC_RESPONSE_SUCCESS: &str = r#"{"id": null, "jsonrpc": "2.0", "result": "dummy"}"#;
pub const JSONRPC_RESPONSE_ERROR: &str =
    r#"{"id": null, "jsonrpc": "2.0", "error": {"code": -32700, "message": "Error message"}}"#;

pub struct WsServer {
    pub worker: Arc<Mutex<Worker>>,
    pub ws_host: String,
    pub ws_port: String,
    pub ws_answer_timeout_ms: u64,
    pub graceful_shutdown: Arc<Mutex<bool>>,
}

impl WsServer {
    pub fn start(self) {
        let _ = task::block_on(Self::run(self));
    }

    fn response_is_error(json_string: &str) -> bool {
        if let Ok(json) = Json::from_str(json_string) {
            if let Some(object) = json.as_object() {
                return object.contains_key("error");
            }
        }

        true
    }

    fn make_io_handler() -> IoHandler {
        let mut io = IoHandler::new();

        io.add_sync_method("coin_average_price", |_params: Params| {
            Ok(Value::String("dummy".to_string()))
        });

        io.add_sync_method("unsubscribe", |_params: Params| {
            Ok(Value::String("ok".to_string()))
        });

        io
    }

    fn parse_json(json_string: String) -> Option<Json> {
        let value = if let Ok(value) = Json::from_str(&json_string) {
            Some(value)
        } else {
            let value = format!("\"{}\"", json_string);
            Json::from_str(&value).ok()
        };

        value
    }

    pub fn fill_response_success(json_string: &mut String, value: String) -> Option<()> {
        let value = Self::parse_json(value)?;

        let mut json = Json::from_str(json_string).ok()?;
        let object = json.as_object_mut()?;

        object.insert("result".to_string(), value);
        *json_string = json.to_string();

        Some(())
    }

    fn fill_response_error(json_string: &mut String, value: String) -> Option<()> {
        let value = Self::parse_json(value)?;

        let mut json = Json::from_str(json_string).ok()?;
        let object = json.as_object_mut()?.get_mut("error")?.as_object_mut()?;

        object.insert("message".to_string(), value);
        *json_string = json.to_string();

        Some(())
    }

    /// What function does:
    /// -- Validate JSON RPC
    /// -- Parse `channel` from json
    /// -- Make response skeleton
    fn preprocess_request(
        request: Message,
    ) -> (serde_json::Result<WsChannelRequest>, Option<String>) {
        let request = request.to_string();
        let channel = serde_json::from_str(&request);

        let io = Self::make_io_handler();
        let response = io.handle_request_sync(&request);

        (channel, response)
    }

    /// Function adds new channel to worker or removes existing channel from worker (depends on `channel`)
    fn add_new_channel(
        worker: Arc<Mutex<Worker>>,
        broadcast_recipient: Tx,
        id: String,
        channel: WsChannelRequest,
        ws_answer_timeout_ms: u64,
    ) {
        if !matches!(channel, WsChannelRequest::Unsubscribe) {
            let response_sender =
                WsChannelResponseSender::new(broadcast_recipient, channel, ws_answer_timeout_ms);

            worker
                .lock()
                .unwrap()
                .pair_average_price
                .add_ws_channel(id, response_sender);
        } else {
            worker
                .lock()
                .unwrap()
                .pair_average_price
                .remove_ws_channel(&id);
        }
    }

    /// What function does:
    /// -- check whether response is ok
    /// -- if response is ok - call `Self::add_new_channel`
    /// -- else - send response
    fn do_response(
        worker: Arc<Mutex<Worker>>,
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        response: Option<String>,
        id: String,
        channel: serde_json::Result<WsChannelRequest>,
        ws_answer_timeout_ms: u64,
    ) {
        if let Some(response) = response {
            match channel {
                Ok(channel) => {
                    info!(
                        "Client with addr: {} subscribed to: {:?}",
                        client_addr, channel
                    );

                    let response_is_error = Self::response_is_error(&response);
                    if !response_is_error {
                        Self::add_new_channel(
                            worker,
                            broadcast_recipient,
                            id,
                            channel,
                            ws_answer_timeout_ms,
                        );
                    } else {
                        let response = Message::from(response);
                        let _ = broadcast_recipient.unbounded_send(response);
                    }
                }
                Err(_) => {
                    let mut response = JSONRPC_RESPONSE_ERROR.to_string();
                    Self::fill_response_error(&mut response, "Channel parse error.".to_string());
                    let response = Message::from(response);
                    let _ = broadcast_recipient.unbounded_send(response);
                }
            }
        } else {
            error!(
                "Handle request error. Client addr: {}, channel: {:?}",
                client_addr, channel
            );
        }
    }

    /// Function calls `Self::preprocess_request`, then starts `Self::do_response` in a separate thread
    fn process_request(
        request: Message,
        worker: &Arc<Mutex<Worker>>,
        peer_map: &PeerMap,
        client_addr: SocketAddr,
        id: &str,
        ws_answer_timeout_ms: u64,
        _graceful_shutdown: &Arc<Mutex<bool>>,
    ) {
        let (channel, response) = Self::preprocess_request(request);

        let peers = peer_map.lock().unwrap();

        if let Some(broadcast_recipient) = peers.get(&client_addr).cloned() {
            let id_2 = id.to_string();
            let worker_2 = Arc::clone(worker);

            let thread_name = format!(
                "fn: process_request, addr: {}, channel: {:?}",
                client_addr, channel
            );
            thread::Builder::new()
                .name(thread_name)
                .spawn(move || {
                    Self::do_response(
                        worker_2,
                        &client_addr,
                        broadcast_recipient,
                        response,
                        id_2,
                        channel,
                        ws_answer_timeout_ms,
                    )
                })
                .unwrap();
        } else {
            // FSR broadcast recipient is not found
        }
    }

    /// Function handles one connection - function is executing until client is disconnected.
    /// Function listens for requests and process them (calls `Self::process_request`)
    async fn handle_connection(
        worker: Arc<Mutex<Worker>>,
        peer_map: PeerMap,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        id: String,
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
                        request,
                        &worker,
                        &peer_map,
                        client_addr,
                        &id,
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
        let server_addr = self.ws_host.clone() + ":" + &self.ws_port;
        let state = PeerMap::new(Mutex::new(HashMap::new()));

        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = TcpListener::bind(&server_addr).await;
        let listener = try_socket.expect("Failed to bind");
        info!("Websocket server started on: {}", server_addr);

        // Let's spawn the handling of each connection in a separate task.
        while let Ok((stream, client_addr)) = listener.accept().await {
            if *self.graceful_shutdown.lock().unwrap() {
                break;
            }

            let id = Uuid::new_v4().to_string();

            let _ = task::spawn(Self::handle_connection(
                Arc::clone(&self.worker),
                state.clone(),
                stream,
                client_addr,
                id,
                self.ws_answer_timeout_ms,
                Arc::clone(&self.graceful_shutdown),
            ));
        }

        Ok(())
    }
}
