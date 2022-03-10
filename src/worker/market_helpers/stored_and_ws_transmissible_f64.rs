use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::market_helpers::hepler_functions::{send_ws_response_1, send_ws_response_2};
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::sync::{Arc, Mutex};

pub struct StoredAndWsTransmissibleF64 {
    value: Option<f64>,
    timestamp: DateTime<Utc>,
    repository: Option<RepositoryForF64ByTimestamp>,
    ws_channels: Arc<Mutex<WsChannels>>,
    ws_channel_names: Vec<WsChannelName>,
    market_name: Option<String>,
    pair: Option<(String, String)>,
}

impl StoredAndWsTransmissibleF64 {
    pub fn new(
        repository: Option<RepositoryForF64ByTimestamp>,
        ws_channel_names: Vec<WsChannelName>,
        market_name: Option<String>,
        pair: Option<(String, String)>,
        ws_channels: Arc<Mutex<WsChannels>>,
    ) -> Self {
        for ws_channel_name in &ws_channel_names {
            if ws_channel_name.is_worker_channel() {
                // Worker's channel

                assert_eq!(market_name, None);
            } else {
                // Market's channel

                assert!(matches!(market_name, Some(..)));
            }

            if matches!(ws_channel_name, WsChannelName::IndexPrice) {
                assert!(pair.is_none());
            } else {
                assert!(pair.is_some());
            }
        }

        Self {
            value: None,
            timestamp: MIN_DATETIME,
            repository,
            ws_channels,
            ws_channel_names,
            market_name,
            pair,
        }
    }

    pub fn get_value(&self) -> Option<f64> {
        self.value
    }

    pub fn set_new_value(&mut self, new_value: f64) {
        self.value = Some(new_value);
        self.timestamp = Utc::now();

        if let Some(repository) = &mut self.repository {
            let _ = repository.insert(self.timestamp, new_value);
        }

        for ws_channel_name in &self.ws_channel_names {
            match ws_channel_name {
                WsChannelName::CoinAveragePrice
                | WsChannelName::CoinExchangePrice
                | WsChannelName::CoinExchangeVolume => {
                    send_ws_response_1(
                        &mut self.ws_channels,
                        *ws_channel_name,
                        &self.market_name,
                        &self.pair,
                        new_value,
                        self.timestamp,
                    );
                }
                WsChannelName::CoinAveragePriceCandles => {
                    send_ws_response_2(
                        &self.repository,
                        &self.ws_channels,
                        *ws_channel_name,
                        &self.pair,
                        self.timestamp,
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
