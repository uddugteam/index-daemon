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

impl Clone for MarketChannels {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for MarketChannels {}
