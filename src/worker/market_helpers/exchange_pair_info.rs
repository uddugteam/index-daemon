use chrono::{DateTime, Utc, MIN_DATETIME};
use std::fmt::{Display, Formatter};

pub struct ExchangePairInfo {
    last_trade_price: f64,
    last_trade_volume: f64,
    volume: f64,
    total_ask: f64,
    total_bid: f64,
    timestamp: DateTime<Utc>,
}

impl ExchangePairInfo {
    pub fn new() -> Self {
        ExchangePairInfo {
            last_trade_price: 0.0,
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

    pub fn set_total_volume(&mut self, value: f64) {
        self.volume = value;
    }
}

impl Clone for ExchangePairInfo {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}

impl Copy for ExchangePairInfo {}

impl Display for ExchangePairInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timestamp:        {}", self.timestamp.to_string())?;
        write!(f, "LastTradePrice:   {}", self.last_trade_price.to_string())?;
        write!(
            f,
            "LastTradeVolume:  {}",
            self.last_trade_volume.to_string()
        )?;
        write!(f, "TotalVolume:      {}", self.volume.to_string())?;
        write!(f, "TotalAsk:         {}", self.total_ask.to_string())?;
        write!(f, "TotalBid:         {}", self.total_bid.to_string())?;

        Ok(())
    }
}
