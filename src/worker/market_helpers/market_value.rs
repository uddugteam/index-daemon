use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MarketValue {
    IndexPrice,
    PairAveragePrice,
    PairExchangePrice,
    PairExchangeVolume,
}

impl FromStr for MarketValue {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "index_price" => Ok(Self::IndexPrice),
            "pair_average_price" => Ok(Self::PairAveragePrice),
            "pair_exchange_price" => Ok(Self::PairExchangePrice),
            "pair_exchange_volume" => Ok(Self::PairExchangeVolume),
            _ => Err(()),
        }
    }
}

impl ToString for MarketValue {
    fn to_string(&self) -> String {
        match self {
            Self::IndexPrice { .. } => "index_price".to_string(),
            Self::PairAveragePrice { .. } => "pair_average_price".to_string(),
            Self::PairExchangePrice { .. } => "pair_exchange_price".to_string(),
            Self::PairExchangeVolume { .. } => "pair_exchange_volume".to_string(),
        }
    }
}
