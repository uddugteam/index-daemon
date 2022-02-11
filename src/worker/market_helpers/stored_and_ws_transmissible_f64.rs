use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::market_helpers::hepler_functions::send_ws_response_1;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};

pub struct StoredAndWsTransmissibleF64 {
    value: f64,
    timestamp: DateTime<Utc>,
    repository: Option<RepositoryForF64ByTimestamp>,
    pub ws_channels: WsChannels,
    ws_channel_name: WsChannelName,
    market_name: Option<String>,
    pair: (String, String),
}

impl StoredAndWsTransmissibleF64 {
    pub fn new(
        repository: Option<RepositoryForF64ByTimestamp>,
        ws_channel_name: WsChannelName,
        market_name: Option<String>,
        pair: (String, String),
    ) -> Self {
        if ws_channel_name.is_worker_channel() {
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

        match self.ws_channel_name {
            WsChannelName::CoinAveragePrice
            | WsChannelName::CoinExchangePrice
            | WsChannelName::CoinExchangeVolume => {
                send_ws_response_1(
                    &mut self.ws_channels,
                    self.ws_channel_name,
                    &self.market_name,
                    &self.pair,
                    new_value,
                    self.timestamp,
                );
            }
            _ => unreachable!(),
        }
    }
}
