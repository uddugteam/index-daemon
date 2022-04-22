use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, strip_usd};
use crate::worker::market_helpers::percent_change::PercentChangeByInterval;
use crate::worker::network_helpers::ws_server::candles::Candle;
use crate::worker::network_helpers::ws_server::channels::worker_channels::LocalWorkerChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct StoredAndWsTransmissibleF64 {
    value: Option<f64>,
    timestamp: DateTime<Utc>,
    percent_change: Arc<RwLock<PercentChangeByInterval>>,
    percent_change_interval_sec: u64,
    pub repository: Option<RepositoryForF64ByTimestamp>,
    ws_channels: Arc<RwLock<WsChannels>>,
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
        percent_change: Arc<RwLock<PercentChangeByInterval>>,
        percent_change_interval_sec: u64,
        ws_channels: Arc<RwLock<WsChannels>>,
    ) -> Self {
        for ws_channel_name in &ws_channel_names {
            if ws_channel_name.is_worker_channel() {
                // Worker's channel

                assert_eq!(market_name, None);
            } else {
                // Market's channel

                assert!(matches!(market_name, Some(..)));
            }

            if matches!(ws_channel_name, WsChannelName::IndexPrice)
                | matches!(ws_channel_name, WsChannelName::IndexPriceCandles)
            {
                assert!(pair.is_none());
            } else {
                assert!(pair.is_some());
            }
        }

        Self {
            value: None,
            timestamp: MIN_DATETIME,
            percent_change,
            percent_change_interval_sec,
            repository,
            ws_channels,
            ws_channel_names,
            market_name,
            pair,
        }
    }

    pub fn get(&self) -> Option<f64> {
        self.value
    }

    pub async fn set(&mut self, value: f64) {
        self.value = Some(value);
        self.timestamp = Utc::now();

        if let Some(repository) = &mut self.repository {
            let _ = repository.insert(self.timestamp, value);
        }

        self.percent_change.write().await.set(value, self.timestamp);

        self.send().await;
    }

    async fn get_percent_change(&self, percent_change_interval_sec: u64) -> Option<f64> {
        self.percent_change
            .read()
            .await
            .get_percent_change(percent_change_interval_sec)
    }

    async fn send(&self) {
        if let Some(value) = self.value {
            for ws_channel_name in &self.ws_channel_names {
                match ws_channel_name {
                    WsChannelName::IndexPrice
                    | WsChannelName::CoinAveragePrice
                    | WsChannelName::CoinExchangePrice
                    | WsChannelName::CoinExchangeVolume => {
                        self.send_ws_response_1(*ws_channel_name, value).await;
                    }
                    WsChannelName::IndexPriceCandles | WsChannelName::CoinAveragePriceCandles => {
                        self.send_ws_response_2(*ws_channel_name).await;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    async fn send_ws_response_1(&self, ws_channel_name: WsChannelName, value: f64) -> Option<()> {
        let market_name = self.market_name.as_ref().map(|v| v.to_string());
        let timestamp = self.timestamp;
        let mut ws_channels = self.ws_channels.write().await;
        let channels = ws_channels.get_channels_by_method(ws_channel_name);
        let mut responses = HashMap::new();

        for (key, request) in channels {
            let percent_change_interval_sec = match request.get_percent_change_interval_sec() {
                Some(v) if v != 0 => v,
                _ => self.percent_change_interval_sec,
            };
            let percent_change = self.get_percent_change(percent_change_interval_sec).await;

            let response_payload = match request.get_method() {
                WsChannelName::IndexPrice => WsChannelResponsePayload::IndexPrice {
                    value,
                    timestamp,
                    percent_change_interval_sec,
                    percent_change,
                },
                WsChannelName::CoinAveragePrice => WsChannelResponsePayload::CoinAveragePrice {
                    coin: strip_usd(self.pair.as_ref()?)?,
                    value,
                    timestamp,
                    percent_change_interval_sec,
                    percent_change,
                },
                WsChannelName::CoinExchangePrice => WsChannelResponsePayload::CoinExchangePrice {
                    coin: strip_usd(self.pair.as_ref()?)?,
                    exchange: market_name.clone().unwrap(),
                    value,
                    timestamp,
                    percent_change_interval_sec,
                    percent_change,
                },
                WsChannelName::CoinExchangeVolume => WsChannelResponsePayload::CoinExchangeVolume {
                    coin: strip_usd(self.pair.as_ref()?)?,
                    exchange: market_name.clone().unwrap(),
                    value,
                    timestamp,
                    percent_change_interval_sec,
                    percent_change,
                },
                _ => unreachable!(),
            };

            responses.insert(key.clone(), response_payload);
        }

        ws_channels.send_individual(responses).await;

        Some(())
    }

    async fn send_ws_response_2(&self, ws_channel_name: WsChannelName) -> Option<()> {
        let repository = self.repository.as_ref()?;
        let mut ws_channels = self.ws_channels.write().await;
        let channels = ws_channels.get_channels_by_method(ws_channel_name);
        let mut responses = HashMap::new();

        for (key, request) in channels {
            match request {
                WsChannelSubscriptionRequest::Worker(channel) => match channel {
                    LocalWorkerChannels::IndexPriceCandles { interval_sec, .. }
                    | LocalWorkerChannels::CoinAveragePriceCandles { interval_sec, .. } => {
                        let to = Utc::now();
                        let from = to.timestamp() as u64 - interval_sec;
                        let from = date_time_from_timestamp_sec(from);

                        if let Ok(values) = repository.read_range(from, to).await {
                            if !values.is_empty() {
                                let value = Candle::calculate(values).unwrap();

                                let response_payload = match channel {
                                    LocalWorkerChannels::IndexPriceCandles { .. } => {
                                        WsChannelResponsePayload::IndexPriceCandles { value }
                                    }
                                    LocalWorkerChannels::CoinAveragePriceCandles { .. } => {
                                        WsChannelResponsePayload::CoinAveragePriceCandles {
                                            coin: strip_usd(self.pair.as_ref()?)?.clone(),
                                            value,
                                        }
                                    }
                                    _ => unreachable!(),
                                };

                                responses.insert(key.clone(), response_payload);
                            }
                        }
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }

        ws_channels.send_individual(responses).await;

        Some(())
    }
}
