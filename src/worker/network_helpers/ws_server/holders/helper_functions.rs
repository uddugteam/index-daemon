use crate::config_scheme::config_scheme::ConfigScheme;
use crate::worker::market_helpers::market_value::MarketValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub type HolderKey = (String, MarketValue, Option<(String, String)>);
pub type HolderHashMap<T> = HashMap<HolderKey, Arc<RwLock<T>>>;

pub fn make_holder_hashmap<T: From<ConfigScheme>>(config: &ConfigScheme) -> HolderHashMap<T> {
    let mut ws_channels_holder = HashMap::new();

    let key = ("worker".to_string(), MarketValue::IndexPrice, None);
    ws_channels_holder.insert(key, Arc::new(RwLock::new(T::from(config.clone()))));

    for exchange_pair in &config.market.exchange_pairs {
        let key = (
            "worker".to_string(),
            MarketValue::PairAveragePrice,
            Some(exchange_pair.clone()),
        );
        ws_channels_holder.insert(key, Arc::new(RwLock::new(T::from(config.clone()))));
    }

    let market_values = [
        MarketValue::PairExchangePrice,
        MarketValue::PairExchangeVolume,
    ];
    for market_name in &config.market.markets {
        for market_value in market_values {
            for exchange_pair in &config.market.exchange_pairs {
                let key = (
                    market_name.to_string(),
                    market_value,
                    Some(exchange_pair.clone()),
                );
                ws_channels_holder.insert(key, Arc::new(RwLock::new(T::from(config.clone()))));
            }
        }
    }

    ws_channels_holder
}
