use std::fmt::{Display, Formatter};

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

impl Display for MarketChannels {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let channel_string = match self {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trades",
            MarketChannels::Book => "book",
        };

        write!(f, "{}", channel_string)
    }
}
impl Clone for MarketChannels {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for MarketChannels {}
