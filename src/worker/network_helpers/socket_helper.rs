use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use flate2::read::GzDecoder;
use futures::{future, pin_mut, StreamExt};
use rustc_serialize::json::Json;
use std::io::prelude::*;

pub struct SocketHelper<F>
where
    F: Fn(String, String),
{
    uri: String,
    on_open_msg: Option<String>,
    pair: String,
    callback: F,
}

impl<F> SocketHelper<F>
where
    F: Fn(String, String),
{
    pub fn new(uri: String, on_open_msg: Option<String>, pair: String, callback: F) -> Self {
        Self {
            uri,
            on_open_msg,
            pair,
            callback,
        }
    }

    pub fn start(self) {
        task::block_on(run(self));
    }
}

async fn run<F>(socket_helper: SocketHelper<F>)
where
    F: Fn(String, String),
{
    match connect_async(&socket_helper.uri).await {
        Ok((ws_stream, _)) => {
            let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();

            if let Some(on_open_msg) = socket_helper.on_open_msg {
                stdin_tx.unbounded_send(Message::text(on_open_msg)).unwrap();
            }

            let (write, read) = ws_stream.split();

            let stdin_to_ws = stdin_rx.map(Ok).forward(write);
            let ws_to_stdout = read.for_each(|message| async {
                if let Ok(message) = message {
                    let message = if let Ok(message) = message.clone().into_text() {
                        Some(message)
                    } else {
                        // unzip (needed for Huobi market)
                        let message = message.into_data();

                        let mut gz_decoder = GzDecoder::new(message.as_slice());
                        let mut message = String::new();

                        if gz_decoder.read_to_string(&mut message).is_ok() {
                            Some(message)
                        } else {
                            None
                        }
                    };

                    if let Some(message) = message {
                        let mut message_is_ping = false;
                        // Check whether message is ping (needed for Huobi market)
                        if let Ok(json) = Json::from_str(&message) {
                            if let Some(object) = json.as_object() {
                                if let Some(value) = object.get("ping") {
                                    if let Some(value) = value.as_u64() {
                                        // Message is ping, thus we need to answer with pong (needed for Huobi market)
                                        message_is_ping = true;
                                        let pong = format!("{{\"pong\":{}}}", value.to_string());
                                        stdin_tx.unbounded_send(Message::text(pong)).unwrap();
                                    }
                                }
                            }
                        }

                        if !message_is_ping {
                            (socket_helper.callback)(socket_helper.pair.clone(), message);
                        }
                    }
                }
            });

            pin_mut!(stdin_to_ws, ws_to_stdout);
            future::select(stdin_to_ws, ws_to_stdout).await;
        }
        Err(e) => error!(
            "Websocket: Failed to connect. Url: {}, pair: {}, error: {:?}",
            socket_helper.uri, socket_helper.pair, e
        ),
    }
}
