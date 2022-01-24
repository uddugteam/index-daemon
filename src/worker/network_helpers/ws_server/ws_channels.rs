use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct WsChannels(HashMap<(String, String), WsChannelResponseSender>);

impl WsChannels {
    pub fn new(ws_channels: HashMap<(String, String), WsChannelResponseSender>) -> Self {
        Self(ws_channels)
    }

    pub fn ws_send(&mut self, pair: (String, String), new_price: f64, timestamp: DateTime<Utc>) {
        let pair = (pair.0.as_str(), pair.1.as_str());
        let coin = match pair {
            ("USD", coin) | (coin, "USD") => {
                // good pair (coin-fiat)
                Some(coin.to_string())
            }
            _ => {
                // bad pair (coin-coin)
                None
            }
        };

        if let Some(coin) = coin {
            let senders: HashMap<&(String, String), &mut WsChannelResponseSender> = self
                .0
                .iter_mut()
                .filter(|(_, v)| v.request.get_coins().contains(&coin))
                .collect();

            let mut keys_to_remove = Vec::new();

            for (key, sender) in senders {
                let response = WsChannelResponse::CoinAveragePrice {
                    id: sender.request.get_id(),
                    coin: coin.clone(),
                    value: new_price,
                    timestamp,
                };
                let send_msg_result = sender.send(response);

                if let Some(send_msg_result) = send_msg_result {
                    if send_msg_result.is_err() {
                        // Send msg error. The client is likely disconnected. We stop sending him messages.

                        keys_to_remove.push(key.clone());
                    }
                } else {
                    // Message wasn't sent because of frequency_ms (not enough time has passed since last dispatch)
                }
            }

            for key in keys_to_remove {
                self.0.remove(&key);
            }
        }
    }

    pub fn add_ws_channel(&mut self, conn_id: String, mut channel: WsChannelResponseSender) {
        let sub_id = channel.request.get_id();
        let method = channel.request.get_method();

        if channel.send_succ_sub_notif(sub_id).is_ok() {
            self.0.insert((conn_id, method), channel);
        } else {
            // Send msg error. The client is likely disconnected. Thus, we don't even establish subscription.
        }
    }

    pub fn remove_ws_channel(&mut self, key: &(String, String)) {
        self.0.remove(key);
    }
}
