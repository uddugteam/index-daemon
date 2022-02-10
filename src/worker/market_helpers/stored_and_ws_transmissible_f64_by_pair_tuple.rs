use crate::repository::repositories::RepositoryForF64ByTimestampAndPairTuple;
use crate::worker::defaults::WS_SERVER_ALL_CHANNELS;
use crate::worker::helper_functions::strip_usd;
use crate::worker::market_helpers::hepler_functions::{prepare_candle_data, send_ws_response_1};
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, NaiveDateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;

pub struct StoredAndWsTransmissibleF64ByPairTuple {
    value: HashMap<(String, String), f64>,
    timestamp: DateTime<Utc>,
    repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
    pub ws_channels: WsChannels,
    ws_channel_names: Vec<String>,
    market_name: Option<String>,
}

impl StoredAndWsTransmissibleF64ByPairTuple {
    pub fn new(
        repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
        ws_channel_names: Vec<String>,
        market_name: Option<String>,
    ) -> Self {
        for ws_channel_name in &ws_channel_names {
            assert!(WS_SERVER_ALL_CHANNELS.contains(&ws_channel_name.as_str()));

            if (ws_channel_name == "coin_average_price")
                | (ws_channel_name == "coin_average_price_candles")
            {
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
            match ws_channel_name.as_str() {
                "coin_average_price" | "coin_exchange_price" | "coin_exchange_volume" => {
                    send_ws_response_1(
                        &mut self.ws_channels,
                        ws_channel_name,
                        &self.market_name,
                        &pair,
                        new_value,
                        self.timestamp,
                    );
                }
                "coin_average_price_candles" => {
                    if let Some(repository) = &self.repository {
                        let channels = self.ws_channels.get_channels_by_method(ws_channel_name);
                        let mut responses = HashMap::new();

                        for (key, request) in channels {
                            let to = self.timestamp;

                            let interval = request.get_interval().into_seconds() as i64;
                            let from = self.timestamp.timestamp() - interval;
                            let from = NaiveDateTime::from_timestamp(from, 0);
                            let from = DateTime::from_utc(from, Utc);

                            if let Ok(values) =
                                repository.read_range((from, pair.clone()), (to, pair.clone()))
                            {
                                let (open, close, min, max, avg) = prepare_candle_data(values);

                                if let Some(coin) = strip_usd(&pair) {
                                    let response_payload =
                                        WsChannelResponsePayload::CoinAveragePriceCandles {
                                            coin,
                                            open,
                                            close,
                                            min,
                                            max,
                                            avg,
                                            timestamp: self.timestamp,
                                        };

                                    responses.insert(key.clone(), response_payload);
                                }
                            }
                        }

                        self.ws_channels.send_individual(responses);
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
