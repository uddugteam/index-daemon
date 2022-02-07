use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::defaults::WS_SERVER_ALL_CHANNELS;
use crate::worker::market_helpers::hepler_functions::send_ws_response;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};

pub struct StoredAndWsTransmissibleF64 {
    value: f64,
    timestamp: DateTime<Utc>,
    repository: Option<RepositoryForF64ByTimestamp>,
    pub ws_channels: WsChannels,
    ws_channel_name: String,
    market_name: Option<String>,
    pair: (String, String),
}

impl StoredAndWsTransmissibleF64 {
    pub fn new(
        repository: Option<RepositoryForF64ByTimestamp>,
        ws_channel_name: String,
        market_name: Option<String>,
        pair: (String, String),
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
            value: 0.0,
            timestamp: MIN_DATETIME,
            repository,
            ws_channels: WsChannels::new(),
            ws_channel_name,
            market_name,
            pair,
        }
    }

    pub fn get_value(&self) -> f64 {
        self.value
    }

    pub fn set_new_value(&mut self, new_value: f64) {
        self.value = new_value;
        self.timestamp = Utc::now();

        if let Some(repository) = &mut self.repository {
            let _ = repository.insert(self.timestamp, new_value);
        }

        send_ws_response(
            &mut self.ws_channels,
            &self.ws_channel_name,
            &self.market_name,
            &self.pair,
            new_value,
            self.timestamp,
        );
    }
}
