use crate::worker::helper_functions::add_jsonrpc_version;
use crate::worker::network_helpers::ws_server::json_rpc_messages::{
    JsonRpcErr, JsonRpcRequest, JsonRpcResponse,
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
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, sync::Mutex, thread};
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

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

    /// What function does:
    /// -- Validate JSON RPC
    /// -- Parse `channel` from json
    /// -- Make response skeleton
    fn preprocess_request(
        request: Message,
    ) -> (
        Option<Result<WsChannelRequest, String>>,
        serde_json::Result<JsonRpcRequest>,
    ) {
        let request = request.to_string();

        let request: serde_json::Result<JsonRpcRequest> = serde_json::from_str(&request);
        let channel = if let Ok(request) = request.as_ref() {
            Some(request.try_into())
        } else {
            None
        };

        (channel, request)
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
            let response_sender =
                WsChannelResponseSender::new(broadcast_recipient, channel, ws_answer_timeout_ms);

            let add_ws_channel_result = worker
                .lock()
                .unwrap()
                .add_ws_channel(conn_id, response_sender);

            // TODO: There we need to check if error and answer to recipient with error message
        } else {
            let method = channel.get_method();

            worker.lock().unwrap().remove_ws_channel(&(conn_id, method));
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
        request: serde_json::Result<JsonRpcRequest>,
        conn_id: String,
        channel: Option<Result<WsChannelRequest, String>>,
        ws_answer_timeout_ms: u64,
    ) {
        match request {
            Ok(request) => {
                // If `request` is `Ok`, then `channel` is `Some` anyway
                let channel = channel.unwrap();

                match channel {
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
                    Err(_) => {
                        let response = JsonRpcResponse::Err {
                            id: request.id,
                            result: JsonRpcErr {
                                code: -32700,
                                message: "Channel parse error.".to_string(),
                            },
                        };
                        let mut response = serde_json::to_string(&response).unwrap();
                        add_jsonrpc_version(&mut response);

                        let response = Message::from(response);
                        let _ = broadcast_recipient.unbounded_send(response);
                    }
                }
            }
            Err(e) => {
                error!(
                    "Handle request error. Client addr: {}, channel: {:?}, error: {:?}",
                    client_addr, channel, e
                );
            }
        }
    }

    /// Function calls `Self::preprocess_request`, then starts `Self::do_response` in a separate thread
    fn process_request(
        request: Message,
        worker: &Arc<Mutex<Worker>>,
        peer_map: &PeerMap,
        client_addr: SocketAddr,
        conn_id: &str,
        ws_answer_timeout_ms: u64,
        _graceful_shutdown: &Arc<Mutex<bool>>,
    ) {
        let (channel, request) = Self::preprocess_request(request);

        let peers = peer_map.lock().unwrap();

        if let Some(broadcast_recipient) = peers.get(&client_addr).cloned() {
            let conn_id_2 = conn_id.to_string();
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
                        request,
                        conn_id_2,
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
                        request,
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
