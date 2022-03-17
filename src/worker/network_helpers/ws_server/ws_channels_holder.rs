use crate::config_scheme::market_config::MarketConfig;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use std::collections::HashMap;
use std::option::Option::Some;
use std::sync::{Arc, Mutex};

pub type WsChannelsHolderKey = (String, MarketValue, Option<(String, String)>);
pub type WsChannelsHolderHashMap = HashMap<WsChannelsHolderKey, Arc<Mutex<WsChannels>>>;

#[derive(Clone)]
pub struct WsChannelsHolder(WsChannelsHolderHashMap);

impl WsChannelsHolder {
    pub fn new(ws_channels_holder: WsChannelsHolderHashMap) -> Self {
        Self(ws_channels_holder)
    }

    pub fn make_hashmap(market_config: &MarketConfig) -> WsChannelsHolderHashMap {
        let mut ws_channels_holder = HashMap::new();

        let key = ("worker".to_string(), MarketValue::IndexPrice, None);
        ws_channels_holder.insert(key, Arc::new(Mutex::new(WsChannels::new())));

        for exchange_pair in &market_config.exchange_pairs {
            let key = (
                "worker".to_string(),
                MarketValue::PairAveragePrice,
                Some(exchange_pair.clone()),
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
                        Some(exchange_pair.clone()),
                    );
                    ws_channels_holder.insert(key, Arc::new(Mutex::new(WsChannels::new())));
                }
            }
        }

        ws_channels_holder
    }

    pub fn contains_key(&self, key: &WsChannelsHolderKey) -> bool {
        self.0.contains_key(key)
    }

    pub fn add(&self, holder_key: &WsChannelsHolderKey, value: (String, WsChannelResponseSender)) {
        if let Some(ws_channels) = self.0.get(holder_key) {
            let (conn_id, response_sender) = value;

            ws_channels
                .lock()
                .unwrap()
                .add_channel(conn_id, response_sender);
        }
    }

    pub fn remove(&self, ws_channels_key: &(String, WsChannelName)) {
        for ws_channels in self.0.values() {
            ws_channels.lock().unwrap().remove_channel(ws_channels_key);
        }
    }
}
