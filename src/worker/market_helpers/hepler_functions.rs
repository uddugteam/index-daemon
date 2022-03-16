use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, strip_usd};
use crate::worker::network_helpers::ws_server::candles::Candle;
use crate::worker::network_helpers::ws_server::channels::worker_channels::WorkerChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub fn send_ws_response_1(
    ws_channels: &Arc<Mutex<WsChannels>>,
    ws_channel_name: WsChannelName,
    market_name: &Option<String>,
    pair: &Option<(String, String)>,
    value: f64,
    timestamp: DateTime<Utc>,
) -> Option<()> {
    let market_name = market_name.as_ref().map(|v| v.to_string());

    let response_payload = match ws_channel_name {
        WsChannelName::IndexPrice => WsChannelResponsePayload::IndexPrice { value, timestamp },
        WsChannelName::CoinAveragePrice => WsChannelResponsePayload::CoinAveragePrice {
            coin: strip_usd(pair.as_ref()?)?,
            value,
            timestamp,
        },
        WsChannelName::CoinExchangePrice => WsChannelResponsePayload::CoinExchangePrice {
            coin: strip_usd(pair.as_ref()?)?,
            exchange: market_name.unwrap(),
            value,
            timestamp,
        },
        WsChannelName::CoinExchangeVolume => WsChannelResponsePayload::CoinExchangeVolume {
            coin: strip_usd(pair.as_ref()?)?,
            exchange: market_name.unwrap(),
            value,
            timestamp,
        },
        _ => unreachable!(),
    };

    ws_channels.lock().unwrap().send_general(response_payload);

    Some(())
}

pub fn send_ws_response_2(
    repository: &Option<RepositoryForF64ByTimestamp>,
    ws_channels: &Arc<Mutex<WsChannels>>,
    ws_channel_name: WsChannelName,
    pair: &Option<(String, String)>,
    timestamp: DateTime<Utc>,
) -> Option<()> {
    let repository = repository.as_ref()?;
    let mut ws_channels = ws_channels.lock().ok()?;
    let channels = ws_channels.get_channels_by_method(ws_channel_name);
    let mut responses = HashMap::new();

    for (key, request) in channels {
        let to = timestamp;

        match request {
            WsChannelSubscriptionRequest::WorkerChannels(channel) => match channel {
                WorkerChannels::IndexPriceCandles { interval, .. }
                | WorkerChannels::CoinAveragePriceCandles { interval, .. } => {
                    let interval = interval.into_seconds() as i64;
                    let from = timestamp.timestamp() - interval;
                    let from = date_time_from_timestamp_sec(from as u64);

                    if let Ok(values) = repository.read_range(from, to) {
                        let values: Vec<(DateTime<Utc>, f64)> =
                            values.into_iter().map(|(k, v)| (k, v)).collect();

                        if !values.is_empty() {
                            let value = Candle::calculate(values, timestamp).unwrap();

                            let response_payload = match channel {
                                WorkerChannels::IndexPriceCandles { .. } => {
                                    WsChannelResponsePayload::IndexPriceCandles { value }
                                }
                                WorkerChannels::CoinAveragePriceCandles { .. } => {
                                    WsChannelResponsePayload::CoinAveragePriceCandles {
                                        coin: strip_usd(pair.as_ref()?)?.clone(),
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

    ws_channels.send_individual(responses);

    Some(())
}
