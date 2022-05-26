#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum MarketValueOwner {
    Worker,
    Market(String),
}
