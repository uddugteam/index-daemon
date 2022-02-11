use std::str::FromStr;

#[derive(Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WsChannelName {
    CoinAveragePrice,
    CoinExchangePrice,
    CoinExchangeVolume,
    CoinAveragePriceHistorical,
    CoinAveragePriceCandles,
    Unsubscribe,
}

impl WsChannelName {
    pub fn is_worker_channel(&self) -> bool {
        match self {
            Self::CoinAveragePrice { .. } | Self::CoinAveragePriceCandles { .. } => true,
            Self::CoinExchangePrice { .. }
            | Self::CoinExchangeVolume { .. }
            | Self::CoinAveragePriceHistorical { .. } => false,
            Self::Unsubscribe => unreachable!(),
        }
    }
}

impl FromStr for WsChannelName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "coin_average_price" => Ok(Self::CoinAveragePrice),
            "coin_exchange_price" => Ok(Self::CoinExchangePrice),
            "coin_exchange_volume" => Ok(Self::CoinExchangeVolume),
            "coin_average_price_historical" => Ok(Self::CoinAveragePriceHistorical),
            "coin_average_price_candles" => Ok(Self::CoinAveragePriceCandles),
            _ => Err(()),
        }
    }
}

impl ToString for WsChannelName {
    fn to_string(&self) -> String {
        match self {
            Self::CoinAveragePrice { .. } => "coin_average_price".to_string(),
            Self::CoinExchangePrice { .. } => "coin_exchange_price".to_string(),
            Self::CoinExchangeVolume { .. } => "coin_exchange_volume".to_string(),
            Self::CoinAveragePriceHistorical { .. } => "coin_average_price_historical".to_string(),
            Self::CoinAveragePriceCandles { .. } => "coin_average_price_candles".to_string(),
            Self::Unsubscribe => unreachable!(),
        }
    }
}
