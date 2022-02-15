use crate::config_scheme::market_config::MarketConfig;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type WsChannelsHolderKey = (String, MarketValue, (String, String));
pub type WsChannelsHolder = HashMap<WsChannelsHolderKey, Arc<Mutex<WsChannels>>>;

pub fn make_ws_channels_holder(market_config: &MarketConfig) -> WsChannelsHolder {
    let mut ws_channels_holder = HashMap::new();

    for exchange_pair in &market_config.exchange_pairs {
        let key = (
            "worker".to_string(),
            MarketValue::PairAveragePrice,
            exchange_pair.pair.clone(),
        );
        ws_channels_holder.insert(key, Arc::new(Mutex::new(WsChannels::new())));
    }

    let market_values = [
        MarketValue::PairExchangePrice,
        MarketValue::PairExchangeVolume,
    ];
    for market_name in &market_config.markets {
        for market_value in market_values {
            for exchange_pair in &market_config.exchange_pairs {
                let key = (
                    market_name.to_string(),
                    market_value,
                    exchange_pair.pair.clone(),
                );
                ws_channels_holder.insert(key, Arc::new(Mutex::new(WsChannels::new())));
            }
        }
    }

    ws_channels_holder
}
