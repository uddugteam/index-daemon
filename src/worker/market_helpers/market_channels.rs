use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum ExternalMarketChannels {
    Ticker,
    Trades,
    Book,
}

impl ExternalMarketChannels {
    pub fn get_all() -> [Self; 3] {
        [Self::Ticker, Self::Trades, Self::Book]
    }
}

impl FromStr for ExternalMarketChannels {
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
