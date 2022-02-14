use crate::repository::repositories::RepositoryForF64ByTimestampAndPairTuple;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, strip_usd};
use crate::worker::network_helpers::ws_server::candles::Candle;
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
    pair: &(String, String),
    new_value: f64,
    timestamp: DateTime<Utc>,
) -> Option<()> {
    if let Some(coin) = strip_usd(pair) {
        let market_name = market_name.as_ref().map(|v| v.to_string());

        let response_payload = match ws_channel_name {
            WsChannelName::CoinAveragePrice => WsChannelResponsePayload::CoinAveragePrice {
                coin,
                value: new_value,
                timestamp,
            },
            WsChannelName::CoinExchangePrice => WsChannelResponsePayload::CoinExchangePrice {
                coin,
                exchange: market_name.unwrap(),
                value: new_value,
                timestamp,
            },
            WsChannelName::CoinExchangeVolume => WsChannelResponsePayload::CoinExchangeVolume {
                coin,
                exchange: market_name.unwrap(),
                value: new_value,
                timestamp,
            },
            _ => unreachable!(),
        };

        ws_channels.lock().unwrap().send_general(response_payload);

        Some(())
    } else {
        None
    }
}

pub fn send_ws_response_2(
    repository: &Option<RepositoryForF64ByTimestampAndPairTuple>,
    ws_channels: &Arc<Mutex<WsChannels>>,
    ws_channel_name: WsChannelName,
    pair: &(String, String),
    timestamp: DateTime<Utc>,
) -> Option<()> {
    if let Some(coin) = strip_usd(pair) {
        if let Some(repository) = &repository {
            if let Ok(mut ws_channels) = ws_channels.lock() {
                let channels = ws_channels.get_channels_by_method(ws_channel_name);
                let mut responses = HashMap::new();

                for (key, request) in channels {
                    let to = timestamp;

                    let interval = request.get_interval().into_seconds() as i64;
                    let from = timestamp.timestamp() - interval;
                    let from = date_time_from_timestamp_sec(from);

                    if let Ok(values) =
                        repository.read_range((from, pair.clone()), (to, pair.clone()))
                    {
                        let values: Vec<(DateTime<Utc>, f64)> =
                            values.into_iter().map(|(k, v)| (k.0, v)).collect();

                        let response_payload = if !values.is_empty() {
                            WsChannelResponsePayload::CoinAveragePriceCandles {
                                coin: coin.clone(),
                                value: Candle::calculate(values, timestamp).unwrap(),
                            }
                        } else {
                            // Send "empty" message

                            WsChannelResponsePayload::Err {
                                method: Some(WsChannelName::CoinAveragePriceCandles),
                                code: 0,
                                message: "No entries found in the requested time interval."
                                    .to_string(),
                            }
                        };

                        responses.insert(key.clone(), response_payload);
                    }
                }

                ws_channels.send_individual(responses);

                Some(())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}
