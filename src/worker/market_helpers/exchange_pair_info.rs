use crate::repository::repositories::MarketRepositoriesByMarketValue;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels_holder::WsChannelsHolderHashMap;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::sync::Arc;

pub struct ExchangePairInfo {
    pub last_trade_price: StoredAndWsTransmissibleF64,
    last_trade_volume: f64,
    pub total_volume: StoredAndWsTransmissibleF64,
    total_ask: f64,
    total_bid: f64,
    timestamp: DateTime<Utc>,
}

impl ExchangePairInfo {
    pub fn new(
        repositories: Option<MarketRepositoriesByMarketValue>,
        ws_channels_holder: &WsChannelsHolderHashMap,
        market_name: String,
        pair: (String, String),
    ) -> Self {
        let mut repositories = repositories.unwrap_or_default();

        ExchangePairInfo {
            last_trade_price: StoredAndWsTransmissibleF64::new(
                repositories.remove(&MarketValue::PairExchangePrice),
                vec![WsChannelName::CoinExchangePrice],
                Some(market_name.clone()),
                pair.clone(),
                Arc::clone(
                    ws_channels_holder
                        .get(&(
                            market_name.clone(),
                            MarketValue::PairExchangePrice,
                            pair.clone(),
                        ))
                        .unwrap(),
                ),
            ),
            last_trade_volume: 0.0,
            total_volume: StoredAndWsTransmissibleF64::new(
                repositories.remove(&MarketValue::PairExchangeVolume),
                vec![WsChannelName::CoinExchangeVolume],
                Some(market_name.clone()),
                pair.clone(),
                Arc::clone(
                    ws_channels_holder
                        .get(&(market_name, MarketValue::PairExchangeVolume, pair))
                        .unwrap(),
                ),
            ),
            total_ask: 0.0,
            total_bid: 0.0,
            timestamp: MIN_DATETIME,
        }
    }

    pub fn get_total_ask(&self) -> f64 {
        self.total_ask
    }
    pub fn set_total_ask(&mut self, value: f64) {
        self.total_ask = value;
        self.timestamp = Utc::now();
    }

    pub fn get_total_bid(&self) -> f64 {
        self.total_bid
    }
    pub fn set_total_bid(&mut self, value: f64) {
        self.total_bid = value;
        self.timestamp = Utc::now();
    }

    pub fn get_last_trade_volume(&self) -> f64 {
        self.last_trade_volume
    }
    pub fn set_last_trade_volume(&mut self, value: f64) {
        self.last_trade_volume = value;
        self.timestamp = Utc::now();
    }
}
