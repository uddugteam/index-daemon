use crate::repository::pair_average_price_cache::PairAveragePriceCache;
use crate::repository::repository::Repository;
use crate::worker::helper_functions::strip_usd;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct PairAveragePrice {
    value: HashMap<(String, String), f64>,
    timestamp: DateTime<Utc>,
    repository: Arc<Mutex<dyn Repository<(String, String), f64> + Send>>,
    pub ws_channels: WsChannels,
}

impl PairAveragePrice {
    pub fn new() -> Self {
        Self {
            value: HashMap::new(),
            timestamp: MIN_DATETIME,
            repository: Arc::new(Mutex::new(PairAveragePriceCache::new())),
            ws_channels: WsChannels::new(),
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

        let coin = strip_usd(&pair);
        if let Some(coin) = coin {
            let response_payload = WsChannelResponsePayload::CoinAveragePrice {
                coin,
                value: new_price,
                timestamp: self.timestamp,
            };

            self.ws_channels.send(response_payload);
        }
    }
}
