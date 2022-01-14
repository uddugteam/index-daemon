use std::str::FromStr;

#[derive(Debug)]
pub enum MarketChannels {
    Ticker,
    Trades,
    Book,
}

impl MarketChannels {
    pub fn get_all() -> [Self; 3] {
        [Self::Ticker, Self::Trades, Self::Book]
    }
}

// TODO: Replace error type with correct
impl FromStr for MarketChannels {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ticker" => Ok(Self::Ticker),
            "trades" => Ok(Self::Trades),
            "book" => Ok(Self::Book),
            _ => Err(()),
        }
    }
}
impl Clone for MarketChannels {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for MarketChannels {}
