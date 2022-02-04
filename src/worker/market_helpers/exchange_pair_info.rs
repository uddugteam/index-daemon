use crate::repository::repositories::RepositoryForF64ByTimestampAndPairTuple;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use chrono::{DateTime, Utc, MIN_DATETIME};

pub struct ExchangePairInfo {
    pub last_trade_price: StoredAndWsTransmissibleF64,
    last_trade_volume: f64,
    volume: f64,
    total_ask: f64,
    total_bid: f64,
    timestamp: DateTime<Utc>,
}

impl ExchangePairInfo {
    pub fn new(
        repository: RepositoryForF64ByTimestampAndPairTuple,
        market_name: String,
        pair: (String, String),
    ) -> Self {
        ExchangePairInfo {
            last_trade_price: StoredAndWsTransmissibleF64::new(
                repository,
                "coin_exchange_price".to_string(),
                Some(market_name),
                pair,
            ),
            last_trade_volume: 0.0,
            volume: 0.0,
            total_ask: 0.0,
            total_bid: 0.0,
            timestamp: MIN_DATETIME,
        }
    }

    pub fn get_total_volume(&self) -> f64 {
        self.volume
    }
    pub fn set_total_volume(&mut self, value: f64, timestamp: DateTime<Utc>) {
        self.volume = value;
        self.timestamp = timestamp;
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

    pub fn set_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.timestamp = timestamp;
    }
}
