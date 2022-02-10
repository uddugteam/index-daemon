use crate::worker::helper_functions::strip_usd;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use chrono::{DateTime, Utc};

pub fn send_ws_response_1(
    ws_channels: &mut WsChannels,
    ws_channel_name: &str,
    market_name: &Option<String>,
    pair: &(String, String),
    new_value: f64,
    timestamp: DateTime<Utc>,
) -> Option<()> {
    let market_name = market_name.as_ref().map(|v| v.to_string());

    if let Some(coin) = strip_usd(pair) {
        let response_payload = match ws_channel_name {
            "coin_average_price" => WsChannelResponsePayload::CoinAveragePrice {
                coin,
                value: new_value,
                timestamp,
            },
            "coin_exchange_price" => WsChannelResponsePayload::CoinExchangePrice {
                coin,
                exchange: market_name.unwrap(),
                value: new_value,
                timestamp,
            },
            "coin_exchange_volume" => WsChannelResponsePayload::CoinExchangeVolume {
                coin,
                exchange: market_name.unwrap(),
                value: new_value,
                timestamp,
            },
            _ => unreachable!(),
        };

        ws_channels.send(response_payload);

        Some(())
    } else {
        None
    }
}
