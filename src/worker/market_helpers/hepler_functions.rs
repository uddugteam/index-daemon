use crate::repository::f64_by_timestamp_and_pair_tuple_sled::TimestampAndPairTuple;
use crate::worker::helper_functions::strip_usd;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub fn send_ws_response_1(
    ws_channels: &mut WsChannels,
    ws_channel_name: WsChannelName,
    market_name: &Option<String>,
    pair: &(String, String),
    new_value: f64,
    timestamp: DateTime<Utc>,
) -> Option<()> {
    let market_name = market_name.as_ref().map(|v| v.to_string());

    if let Some(coin) = strip_usd(pair) {
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

        ws_channels.send_general(response_payload);

        Some(())
    } else {
        None
    }
}

pub fn prepare_candle_data(
    values: HashMap<TimestampAndPairTuple, f64>,
) -> (f64, f64, f64, f64, f64) {
    let mut values: Vec<(DateTime<Utc>, f64)> = values.into_iter().map(|(k, v)| (k.0, v)).collect();
    values.sort_by(|a, b| a.0.cmp(&b.0));

    let open = values.first().unwrap().1;
    let close = values.last().unwrap().1;
    let mut min = values.first().unwrap().1;
    let mut max = values.first().unwrap().1;

    let mut sum = 0.0;
    let count = values.len();

    for (_, value) in values {
        if value < min {
            min = value;
        }

        if value > max {
            max = value;
        }

        sum += value;
    }

    let avg = sum / count as f64;

    (open, close, min, max, avg)
}
