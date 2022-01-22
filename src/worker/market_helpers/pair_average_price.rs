use crate::repository::pair_average_price_cache::PairAveragePriceCache;
use crate::repository::repository::Repository;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct PairAveragePrice {
    value: HashMap<(String, String), f64>,
    timestamp: DateTime<Utc>,
    repository: Arc<Mutex<dyn Repository<(String, String), f64> + Send>>,
    ws_channels: HashMap<String, WsChannelResponseSender>,
}

impl PairAveragePrice {
    pub fn new() -> Self {
        Self {
            value: HashMap::new(),
            timestamp: MIN_DATETIME,
            repository: Arc::new(Mutex::new(PairAveragePriceCache::new())),
            ws_channels: HashMap::new(),
        }
    }

    fn ws_send(&mut self, pair: (String, String), new_price: f64) {
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
            let senders: HashMap<&String, &mut WsChannelResponseSender> = self
                .ws_channels
                .iter_mut()
                .filter(|(_, v)| v.get_coins().contains(&coin))
                .collect();

            let mut ids_to_remove = Vec::new();

            for (id, sender) in senders {
                let response = WsChannelResponse::CoinAveragePrice {
                    id: sender.get_id(),
                    coin: coin.clone(),
                    value: new_price,
                    timestamp: self.timestamp,
                };
                let send_msg_result = sender.send(response);

                if let Some(send_msg_result) = send_msg_result {
                    if send_msg_result.is_err() {
                        // Send msg error. The client is likely disconnected. We stop sending him messages.

                        ids_to_remove.push(id.to_string());
                    }
                } else {
                    // Message wasn't sent because of frequency_ms (not enough time has passed since last dispatch)
                }
            }

            for id in ids_to_remove {
                self.ws_channels.remove(&id);
            }
        }
    }

    pub fn get_price(&self, pair: &(String, String)) -> Option<f64> {
        self.value.get(pair).cloned()
    }

    /// TODO: Store `timestamp` in `repository`
    pub fn set_new_price(&mut self, pair: (String, String), new_price: f64) {
        self.value.insert(pair.clone(), new_price);
        self.timestamp = Utc::now();

        self.repository
            .lock()
            .unwrap()
            .insert(pair.clone(), new_price);

        self.ws_send(pair, new_price);
    }

    pub fn add_ws_channel(&mut self, id: String, mut channel: WsChannelResponseSender) {
        if channel.send_succ_sub_notif().is_ok() {
            self.ws_channels.insert(id, channel);
        } else {
            // Send msg error. The client is likely disconnected. We don't even establish subscription.
        }
    }

    pub fn remove_ws_channel(&mut self, id: &str) {
        self.ws_channels.remove(id);
    }
}
