use crate::repository::f64_by_timestamp_and_pair_tuple_sled::TimestampAndPairTuple;
use crate::repository::repository::Repository;
use crate::worker::defaults::WS_SERVER_ALL_CHANNELS;
use crate::worker::market_helpers::hepler_functions::send_ws_response;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;

pub struct StoredAndWsTransmissibleF64ByPairTuple {
    value: HashMap<(String, String), f64>,
    timestamp: DateTime<Utc>,
    repository: Box<dyn Repository<TimestampAndPairTuple, f64> + Send>,
    pub ws_channels: WsChannels,
    ws_channel_name: String,
    market_name: Option<String>,
}

impl StoredAndWsTransmissibleF64ByPairTuple {
    pub fn new(
        repository: Box<dyn Repository<TimestampAndPairTuple, f64> + Send>,
        ws_channel_name: String,
        market_name: Option<String>,
    ) -> Self {
        assert!(WS_SERVER_ALL_CHANNELS.contains(&ws_channel_name.as_str()));

        Self {
            value: HashMap::new(),
            timestamp: MIN_DATETIME,
            repository,
            ws_channels: WsChannels::new(),
            ws_channel_name,
            market_name,
        }
    }

    pub fn get_value(&self, pair: &(String, String)) -> Option<f64> {
        self.value.get(pair).cloned()
    }

    pub fn set_new_value(&mut self, pair: (String, String), new_value: f64) {
        self.value.insert(pair.clone(), new_value);
        self.timestamp = Utc::now();

        let _ = self
            .repository
            .insert((self.timestamp, pair.clone()), new_value);

        send_ws_response(
            &mut self.ws_channels,
            &self.ws_channel_name,
            &self.market_name,
            &pair,
            new_value,
            self.timestamp,
        );
    }
}
