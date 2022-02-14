use crate::worker::market_helpers::market_value::MarketValue;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WsChannelName {
    CoinAveragePrice,
    CoinExchangePrice,
    CoinExchangeVolume,
    CoinAveragePriceHistorical,
    CoinAveragePriceCandles,
    CoinAveragePriceCandlesHistorical,
    Unsubscribe,
}

impl WsChannelName {
    pub fn is_worker_channel(&self) -> bool {
        match self {
            Self::CoinAveragePrice { .. } | Self::CoinAveragePriceCandles { .. } => true,
            Self::CoinExchangePrice { .. }
            | Self::CoinExchangeVolume { .. }
            | Self::CoinAveragePriceHistorical { .. }
            | Self::CoinAveragePriceCandlesHistorical { .. } => false,
            Self::Unsubscribe => unreachable!(),
        }
    }

    pub fn is_channel(&self) -> bool {
        match self {
            Self::CoinAveragePrice { .. }
            | Self::CoinExchangePrice { .. }
            | Self::CoinExchangeVolume { .. }
            | Self::CoinAveragePriceCandles { .. }
            | Self::Unsubscribe { .. } => true,
            Self::CoinAveragePriceHistorical { .. }
            | Self::CoinAveragePriceCandlesHistorical { .. } => false,
        }
    }

    pub fn get_market_value(&self) -> MarketValue {
        match self {
            Self::CoinExchangePrice { .. } => MarketValue::PairExchangePrice,
            Self::CoinExchangeVolume { .. } => MarketValue::PairExchangeVolume,
            Self::CoinAveragePrice { .. }
            | Self::CoinAveragePriceHistorical { .. }
            | Self::CoinAveragePriceCandles { .. }
            | Self::CoinAveragePriceCandlesHistorical { .. } => MarketValue::PairAveragePrice,
            Self::Unsubscribe { .. } => unreachable!(),
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
            "coin_average_price_candles_historical" => Ok(Self::CoinAveragePriceCandlesHistorical),
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
            Self::CoinAveragePriceCandlesHistorical { .. } => {
                "coin_average_price_candles_historical".to_string()
            }
            Self::Unsubscribe => unreachable!(),
        }
    }
}
