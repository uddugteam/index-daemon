use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time;

use futures::channel::mpsc::unbounded;
use futures::channel::mpsc::UnboundedSender;
use futures::future;
use futures::pin_mut;
use futures::prelude::*;

use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct WsServer {
    pub ws_host: String,
    pub ws_port: String,
    pub ws_answer_timeout_sec: u64,
}

impl WsServer {
    pub fn start(self) {
        let _ = task::block_on(Self::run(self));
    }

    fn make_thread_name_for_request(client_addr: &SocketAddr, msg: &Message) -> String {
        // There we should parse json to get request params
        format!(
            "fn: process_request, addr: {} with params: {}",
            client_addr, "subscr_params"
        )
    }

    fn process_request(
        client_addr: &SocketAddr,
        broadcast_recipient: Tx,
        msg: Message,
        ws_answer_timeout_sec: u64,
    ) {
        // There we should parse json to get request params
        info!(
            "Client with addr: {} subscribed with params: {}",
            client_addr, "subscr_params"
        );

        loop {
            // Answer with the same message
            if broadcast_recipient.unbounded_send(msg.clone()).is_ok() {
                thread::sleep(time::Duration::from_millis(ws_answer_timeout_sec));
            } else {
                // Send msg error. The client is likely disconnected. We stop sending him messages.
                break;
            }
        }
    }

    async fn handle_connection(
        peer_map: PeerMap,
        raw_stream: TcpStream,
        client_addr: SocketAddr,
        ws_answer_timeout_sec: u64,
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
                    .try_filter(|msg| {
                        // Broadcasting a Close message from one client
                        // will close the other clients.
                        future::ready(!msg.is_close())
                    })
                    .try_for_each(|msg| {
                        let mut peers = peer_map.lock().unwrap();

                        // Change "remove" to "get" when multiple messages within one connection will be allowed
                        if let Some(broadcast_recipient) = peers.remove(&client_addr) {
                            let thread_name =
                                Self::make_thread_name_for_request(&client_addr, &msg);
                            thread::Builder::new()
                                .name(thread_name)
                                .spawn(move || {
                                    Self::process_request(
                                        &client_addr,
                                        broadcast_recipient,
                                        msg,
                                        ws_answer_timeout_sec,
                                    )
                                })
                                .unwrap();
                        } else {
                            // FSR broadcast recipient is not found. Likely he sent two messages,
                            // while only one message within one connection is allowed.
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
            let _ = task::spawn(Self::handle_connection(
                state.clone(),
                stream,
                client_addr,
                self.ws_answer_timeout_sec,
            ));
        }

        Ok(())
    }
}
