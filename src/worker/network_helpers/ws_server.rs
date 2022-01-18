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
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, sync::Mutex, thread, time};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct WsServer {
    pub ws_host: String,
    pub ws_port: String,
    pub ws_answer_timeout_sec: u64,
    pub graceful_shutdown: Arc<Mutex<bool>>,
}

impl WsServer {
    pub fn start(self) {
        let _ = task::block_on(Self::run(self));
    }

    fn parse_request_params(request: &str) -> Option<Json> {
        let json = Json::from_str(request).ok()?;
        let params = json.as_object()?.get("params").cloned();

        params
    }

    // TODO: Implement
    fn fill_response(response: &mut String, value: Json) {}

    fn make_io_handler() -> IoHandler {
        let mut io = IoHandler::new();
        io.add_sync_method("say_hello", |_params: Params| {
            Ok(Value::String("dummy".to_string()))
        });

        io
    }

    fn process_request(
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        request: String,
        request_params: String,
        ws_answer_timeout_sec: u64,
        graceful_shutdown: Arc<Mutex<bool>>,
    ) {
        info!(
            "Client with addr: {} subscribed with params: {}",
            client_addr, request_params
        );

        let io = Self::make_io_handler();

        if let Some(mut response) = io.handle_request_sync(&request) {
            // TODO: Replace dummy with real value
            let response_json = Json::from_str("{}").unwrap();
            Self::fill_response(&mut response, response_json);
            let response = Message::from(response);

            loop {
                if *graceful_shutdown.lock().unwrap() {
                    return;
                }

                if broadcast_recipient.unbounded_send(response.clone()).is_ok() {
                    thread::sleep(time::Duration::from_millis(ws_answer_timeout_sec));
                } else {
                    // Send msg error. The client is likely disconnected. We stop sending him messages.
                    break;
                }
            }
        } else {
            error!(
                "Handle request error. Client addr: {}, params: {}",
                client_addr, request_params
            );
        }
    }

    async fn handle_connection(
        peer_map: PeerMap,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        ws_answer_timeout_sec: u64,
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

                let broadcast_incoming = incoming
                    .try_filter(|request| {
                        // Broadcasting a Close message from one client
                        // will close the other clients.
                        future::ready(!request.is_close())
                    })
                    .try_for_each(|request| {
                        if *graceful_shutdown.lock().unwrap() {
                            return future::ok(());
                        }

                        let request = request.to_string();

                        if let Some(request_params) = Self::parse_request_params(&request) {
                            let request_params = request_params.to_string();
                            let mut peers = peer_map.lock().unwrap();

                            // Change "remove" to "get" when multiple messages within one connection will be allowed
                            if let Some(broadcast_recipient) = peers.remove(&client_addr) {
                                let graceful_shutdown_2 = Arc::clone(&graceful_shutdown);
                                let thread_name = format!(
                                    "fn: process_request, addr: {} with params: {}",
                                    client_addr, request_params
                                );
                                thread::Builder::new()
                                    .name(thread_name)
                                    .spawn(move || {
                                        Self::process_request(
                                            &client_addr,
                                            broadcast_recipient,
                                            request,
                                            request_params,
                                            ws_answer_timeout_sec,
                                            graceful_shutdown_2,
                                        )
                                    })
                                    .unwrap();
                            } else {
                                // FSR broadcast recipient is not found. Likely he sent two messages,
                                // while only one message within one connection is allowed.
                            }
                        } else {
                            // Requests with no params are forbidden
                        }

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

            let _ = task::spawn(Self::handle_connection(
                state.clone(),
                stream,
                client_addr,
                self.ws_answer_timeout_sec,
                Arc::clone(&self.graceful_shutdown),
            ));
        }

        Ok(())
    }
}
