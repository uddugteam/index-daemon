use crate::repository::repositories::RepositoryForF64ByTimestampAndPairTuple;
use crate::worker::market_helpers::hepler_functions::{send_ws_response_1, send_ws_response_2};
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;

pub struct StoredAndWsTransmissibleF64ByPairTuple {
    value: HashMap<(String, String), f64>,
    timestamp: DateTime<Utc>,
    repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
    pub ws_channels: WsChannels,
    ws_channel_names: Vec<WsChannelName>,
    market_name: Option<String>,
}

impl StoredAndWsTransmissibleF64ByPairTuple {
    pub fn new(
        repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
        ws_channel_names: Vec<WsChannelName>,
        market_name: Option<String>,
    ) -> Self {
        for ws_channel_name in &ws_channel_names {
            if ws_channel_name.is_worker_channel() {
                // Worker's channel

                assert_eq!(market_name, None);
            } else {
                // Market's channel

                assert!(matches!(market_name, Some(..)));
            }
        }

        Self {
            value: HashMap::new(),
            timestamp: MIN_DATETIME,
            repository,
            ws_channels: WsChannels::new(),
            ws_channel_names,
            market_name,
        }
    }

    pub fn get_value(&self, pair: &(String, String)) -> Option<f64> {
        self.value.get(pair).cloned()
    }

    pub fn set_new_value(&mut self, pair: (String, String), new_value: f64) {
        self.value.insert(pair.clone(), new_value);
        self.timestamp = Utc::now();

        if let Some(repository) = &mut self.repository {
            let _ = repository.insert((self.timestamp, pair.clone()), new_value);
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
                        &pair,
                        new_value,
                        self.timestamp,
                    );
                }
                WsChannelName::CoinAveragePriceCandles => {
                    send_ws_response_2(
                        &self.repository,
                        &mut self.ws_channels,
                        *ws_channel_name,
                        &pair,
                        self.timestamp,
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
