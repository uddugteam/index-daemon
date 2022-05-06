use crate::worker::market_helpers::market::{debug_write_json_to_file, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use flate2::read::DeflateDecoder;
use flate2::read::GzDecoder;
use futures::channel::mpsc::UnboundedSender;
use futures::{future, pin_mut, StreamExt};
use std::io::prelude::Read;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

type Tx = UnboundedSender<Message>;

pub struct WsClient {
    uri: String,
    on_open_msg: Option<String>,
    pair: String,
    market: Arc<Mutex<dyn Market + Send + Sync>>,
    channel: ExternalMarketChannels,
}

impl WsClient {
    pub fn new(
        uri: String,
        on_open_msg: Option<String>,
        pair: String,
        market: Arc<Mutex<dyn Market + Send + Sync>>,
        channel: ExternalMarketChannels,
    ) -> Self {
        Self {
            uri,
            on_open_msg,
            pair,
            market,
            channel,
        }
    }

    fn unzip<T: Read>(mut decoder: T) -> Option<String> {
        let mut message = String::new();

        if decoder.read_to_string(&mut message).is_ok() {
            Some(message)
        } else {
            None
        }
    }

    /// Checks whether message is "ping" and if it is, answers with "pong" (needed for Huobi market)
    fn send_pong(stdin_tx: &Tx, message: &str) -> Option<()> {
        let json: serde_json::Value = serde_json::from_str(message).ok()?;
        let value = json.as_object()?.get("ping")?.as_u64()?;

        let pong = format!("{{\"pong\":{}}}", value);
        stdin_tx.unbounded_send(Message::text(pong)).unwrap();

        Some(())
    }

    pub async fn run(self) {
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
                            let message = message.as_slice();
                            if let Some(message) = Self::unzip(GzDecoder::new(message)) {
                                Some(message)
                            } else {
                                // unzip deflate (needed for Okcoin market)
                                let message = Self::unzip(DeflateDecoder::new(message));
                                if message.is_some() {
                                    market_is_okcoin = true;
                                }

                                message
                            }
                        };

                        if let Some(message) = message {
                            let message_is_ping = Self::send_pong(&stdin_tx, &message).is_some();
                            if !message_is_ping {
                                if let Ok(json) = serde_json::from_str(&message) {
                                    // // TODO: Remove debug
                                    // if true {
                                    //     debug_write_json_to_file(
                                    //         &self.market.lock().await.get_spine().name,
                                    //         self.channel,
                                    //         &json,
                                    //     );
                                    // }

                                    let pair = self.pair.clone();
                                    let mut market = self.market.lock().await;

                                    match self.channel {
                                        ExternalMarketChannels::Ticker => {
                                            market.parse_ticker_json(pair, json).await;
                                        }
                                        ExternalMarketChannels::Trades => {
                                            market.parse_last_trade_json(pair, json).await;
                                        }
                                        ExternalMarketChannels::Book => {
                                            market.parse_depth_json(pair, json).await;
                                        }
                                    }
                                } else {
                                    // Either parse json error or received string is not json
                                }
                            }

                            if market_is_okcoin {
                                // There we wait a while and then re-send an on_open_msg
                                sleep(Duration::from_millis(3000)).await;
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
