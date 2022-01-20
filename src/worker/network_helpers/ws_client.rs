use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use flate2::read::DeflateDecoder;
use flate2::read::GzDecoder;
use futures::{future, pin_mut, StreamExt};
use rustc_serialize::json::Json;
use std::io::prelude::Read;
use std::thread;
use std::time;

pub struct WsClient<F>
where
    F: Fn(String, String),
{
    uri: String,
    on_open_msg: Option<String>,
    pair: String,
    callback: F,
}

impl<F> WsClient<F>
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
        let _ = task::block_on(Self::run(self));
    }

    fn unzip<T: Read>(mut decoder: T) -> Option<String> {
        let mut message = String::new();

        if decoder.read_to_string(&mut message).is_ok() {
            Some(message)
        } else {
            None
        }
    }

    async fn run(self) {
        match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => {
                let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();

                let on_open_msg = self.on_open_msg.clone();
                if let Some(on_open_msg) = self.on_open_msg {
                    stdin_tx.unbounded_send(Message::text(on_open_msg)).unwrap();
                }

                let (write, read) = ws_stream.split();

                let stdin_to_ws = stdin_rx.map(Ok).forward(write);
                let ws_to_stdout = read.for_each(|message| async {
                    if let Ok(message) = message {
                        let mut market_is_okcoin = false;

                        let message = if let Ok(message) = message.clone().into_text() {
                            Some(message)
                        } else {
                            // unzip gz (needed for Huobi market)
                            let message = message.into_data();
                            if let Some(message) = Self::unzip(GzDecoder::new(message.as_slice())) {
                                Some(message)
                            } else {
                                // unzip deflate (needed for Okcoin market)
                                let message = Self::unzip(DeflateDecoder::new(message.as_slice()));
                                if message.is_some() {
                                    market_is_okcoin = true;
                                }

                                message
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
                                            let pong = format!("{{\"pong\":{}}}", value);
                                            stdin_tx.unbounded_send(Message::text(pong)).unwrap();
                                        }
                                    }
                                }
                            }

                            if !message_is_ping {
                                (self.callback)(self.pair.clone(), message);
                            }

                            if market_is_okcoin {
                                // There we wait a while and then re-send an on_open_msg
                                thread::sleep(time::Duration::from_millis(3000));
                                stdin_tx
                                    .unbounded_send(Message::text(on_open_msg.as_ref().unwrap()))
                                    .unwrap();
                            }
                        }
                    }
                });

                pin_mut!(stdin_to_ws, ws_to_stdout);
                future::select(stdin_to_ws, ws_to_stdout).await;
            }
            Err(e) => error!(
                "Websocket: Failed to connect. Url: {}, pair: {}, error: {:?}",
                self.uri, self.pair, e
            ),
        }
    }
}
