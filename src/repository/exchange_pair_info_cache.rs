use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfoTrait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct ExchangePairInfoCache(HashMap<String, String>);

impl ExchangePairInfoCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl ExchangePairInfoTrait for ExchangePairInfoCache {
    fn get_total_volume(&self) -> f64 {
        self.0
            .get("total_volume")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0.0)
    }

    fn set_total_volume(&mut self, value: f64) {
        self.0.insert("total_volume".to_string(), value.to_string());
    }

    fn get_total_ask(&self) -> f64 {
        self.0
            .get("total_ask")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0.0)
    }

    fn set_total_ask(&mut self, value: f64) {
        self.0.insert("total_ask".to_string(), value.to_string());
    }

    fn get_total_bid(&self) -> f64 {
        self.0
            .get("total_bid")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0.0)
    }

    fn set_total_bid(&mut self, value: f64) {
        self.0.insert("total_bid".to_string(), value.to_string());
    }

    fn get_last_trade_volume(&self) -> f64 {
        self.0
            .get("total_last_trade_volume")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0.0)
    }

    fn set_last_trade_volume(&mut self, value: f64) {
        self.0
            .insert("total_last_trade_volume".to_string(), value.to_string());
    }

    fn get_last_trade_price(&self) -> f64 {
        self.0
            .get("total_last_trade_price")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0.0)
    }

    fn set_last_trade_price(&mut self, value: f64) {
        self.0
            .insert("total_last_trade_price".to_string(), value.to_string());
    }

    fn set_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.0
            .insert("total_timestamp".to_string(), timestamp.to_string());
    }
}
