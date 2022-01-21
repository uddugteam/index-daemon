use crate::repository::exchange_pair_info_cache::ExchangePairInfoCache;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::sync::{Arc, Mutex};

pub trait ExchangePairInfoTrait: Send {
    fn get_total_volume(&self) -> f64;
    fn set_total_volume(&mut self, value: f64);

    fn get_total_ask(&self) -> f64;
    fn set_total_ask(&mut self, value: f64);

    fn get_total_bid(&self) -> f64;
    fn set_total_bid(&mut self, value: f64);

    fn get_last_trade_volume(&self) -> f64;
    fn set_last_trade_volume(&mut self, value: f64);

    fn get_last_trade_price(&self) -> f64;
    fn set_last_trade_price(&mut self, value: f64);

    fn set_timestamp(&mut self, timestamp: DateTime<Utc>);
}

pub struct ExchangePairInfo {
    last_trade_price: f64,
    last_trade_volume: f64,
    volume: f64,
    total_ask: f64,
    total_bid: f64,
    timestamp: DateTime<Utc>,
    repository: Arc<Mutex<dyn ExchangePairInfoTrait>>,
}

impl ExchangePairInfo {
    pub fn new() -> Self {
        let repository = Arc::new(Mutex::new(ExchangePairInfoCache::new()));

        ExchangePairInfo {
            last_trade_price: 0.0,
            last_trade_volume: 0.0,
            volume: 0.0,
            total_ask: 0.0,
            total_bid: 0.0,
            timestamp: MIN_DATETIME,
            repository,
        }
    }
}

impl ExchangePairInfoTrait for ExchangePairInfo {
    fn get_total_volume(&self) -> f64 {
        self.volume
    }
    fn set_total_volume(&mut self, value: f64) {
        self.volume = value;
        self.timestamp = Utc::now();

        self.repository
            .lock()
            .unwrap()
            .set_total_volume(self.volume);
        self.repository
            .lock()
            .unwrap()
            .set_timestamp(self.timestamp);
    }

    fn get_total_ask(&self) -> f64 {
        self.total_ask
    }
    fn set_total_ask(&mut self, value: f64) {
        self.total_ask = value;
        self.timestamp = Utc::now();
    }

    fn get_total_bid(&self) -> f64 {
        self.total_bid
    }
    fn set_total_bid(&mut self, value: f64) {
        self.total_bid = value;
        self.timestamp = Utc::now();
    }

    fn get_last_trade_volume(&self) -> f64 {
        self.last_trade_volume
    }
    fn set_last_trade_volume(&mut self, value: f64) {
        self.last_trade_volume = value;
        self.timestamp = Utc::now();
    }

    fn get_last_trade_price(&self) -> f64 {
        self.last_trade_price
    }
    fn set_last_trade_price(&mut self, value: f64) {
        self.last_trade_price = value;
        self.timestamp = Utc::now();
    }

    fn set_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.timestamp = timestamp;
    }
}
