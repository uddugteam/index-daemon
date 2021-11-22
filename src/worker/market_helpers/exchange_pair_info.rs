use std::fmt::{Display, Formatter};

pub struct ExchangePairInfo {
    last_trade_price: Option<f64>,
    last_trade_volume: Option<f64>,
    volume: Option<f64>,
    total_ask: Option<f64>,
    total_bid: Option<f64>,
    timestamp: Option<chrono::Utc>,
}

impl ExchangePairInfo {
    pub fn new() -> Self {
        ExchangePairInfo {
            last_trade_price: None,
            last_trade_volume: None,
            volume: None,
            total_ask: None,
            total_bid: None,
            timestamp: None,
        }
    }
}

impl Display for ExchangePairInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.last_trade_price.is_none()
            || self.last_trade_volume.is_none()
            || self.volume.is_none()
            || self.total_ask.is_none()
            || self.total_bid.is_none()
            || self.timestamp.is_none()
        {
            Err(std::fmt::Error {})
        } else {
            write!(
                f,
                "Timestamp:        {}",
                self.timestamp.unwrap().to_string()
            )?;
            write!(
                f,
                "LastTradePrice:   {}",
                self.last_trade_price.unwrap().to_string()
            )?;
            write!(
                f,
                "LastTradeVolume:  {}",
                self.last_trade_volume.unwrap().to_string()
            )?;
            write!(f, "TotalVolume:      {}", self.volume.unwrap().to_string())?;
            write!(
                f,
                "TotalAsk:         {}",
                self.total_ask.unwrap().to_string()
            )?;
            write!(
                f,
                "TotalBid:         {}",
                self.total_bid.unwrap().to_string()
            )?;

            Ok(())
        }
    }
}
