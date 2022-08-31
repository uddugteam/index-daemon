#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum MarketValueOwner {
    Worker,
    Market(String),
}

impl ToString for MarketValueOwner {
    fn to_string(&self) -> String {
        match self {
            Self::Worker => "worker".to_string(),
            Self::Market(market_name) => format!("market__{}", market_name),
        }
    }
}
