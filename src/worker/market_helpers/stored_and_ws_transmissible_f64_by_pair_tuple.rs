use crate::repository::repositories::RepositoryForF64ByTimestampAndPairTuple;
use crate::worker::defaults::WS_SERVER_ALL_CHANNELS;
use crate::worker::market_helpers::hepler_functions::send_ws_response;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;

pub struct StoredAndWsTransmissibleF64ByPairTuple {
    value: HashMap<(String, String), f64>,
    timestamp: DateTime<Utc>,
    repository: RepositoryForF64ByTimestampAndPairTuple,
    pub ws_channels: WsChannels,
    ws_channel_name: String,
    market_name: Option<String>,
}

impl StoredAndWsTransmissibleF64ByPairTuple {
    pub fn new(
        repository: RepositoryForF64ByTimestampAndPairTuple,
        ws_channel_name: String,
        market_name: Option<String>,
    ) -> Self {
        assert!(WS_SERVER_ALL_CHANNELS.contains(&ws_channel_name.as_str()));
        if ws_channel_name == "coin_average_price" {
            // Worker's channel

            assert_eq!(market_name, None);
        } else {
            // Market's channel

            assert!(matches!(market_name, Some(..)));
        }

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
