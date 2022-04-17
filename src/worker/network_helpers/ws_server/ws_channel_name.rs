use crate::worker::market_helpers::market_value::MarketValue;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WsChannelName {
    AvailableCoins,
    IndexPrice,
    IndexPriceCandles,
    CoinAveragePrice,
    CoinAveragePriceCandles,
    CoinExchangePrice,
    CoinExchangeVolume,
    IndexPriceHistorical,
    IndexPriceCandlesHistorical,
    CoinAveragePriceHistorical,
    CoinAveragePriceCandlesHistorical,
    Unsubscribe,
}

impl WsChannelName {
    pub fn is_worker_channel(&self) -> bool {
        match self {
            Self::AvailableCoins { .. }
            | Self::IndexPrice { .. }
            | Self::IndexPriceCandles { .. }
            | Self::CoinAveragePrice { .. }
            | Self::CoinAveragePriceCandles { .. } => true,
            Self::CoinExchangePrice { .. }
            | Self::CoinExchangeVolume { .. }
            | Self::IndexPriceHistorical { .. }
            | Self::IndexPriceCandlesHistorical { .. }
            | Self::CoinAveragePriceHistorical { .. }
            | Self::CoinAveragePriceCandlesHistorical { .. } => false,
            Self::Unsubscribe => unreachable!(),
        }
    }

    pub fn is_channel(&self) -> bool {
        match self {
            WsChannelName::IndexPrice
            | WsChannelName::IndexPriceCandles
            | WsChannelName::CoinAveragePrice
            | WsChannelName::CoinAveragePriceCandles
            | WsChannelName::CoinExchangePrice
            | WsChannelName::CoinExchangeVolume => true,
            WsChannelName::AvailableCoins
            | WsChannelName::IndexPriceHistorical
            | WsChannelName::IndexPriceCandlesHistorical
            | WsChannelName::CoinAveragePriceHistorical
            | WsChannelName::CoinAveragePriceCandlesHistorical => false,
            _ => unreachable!(),
        }
    }

    pub fn get_all_channels() -> Vec<Self> {
        vec![
            Self::IndexPrice,
            Self::IndexPriceCandles,
            Self::CoinAveragePrice,
            Self::CoinAveragePriceCandles,
            Self::CoinExchangePrice,
            Self::CoinExchangeVolume,
        ]
    }

    pub fn get_all_methods() -> Vec<Self> {
        vec![
            Self::AvailableCoins,
            Self::IndexPriceHistorical,
            Self::IndexPriceCandlesHistorical,
            Self::CoinAveragePriceHistorical,
            Self::CoinAveragePriceCandlesHistorical,
        ]
    }

    pub fn get_market_value(&self) -> MarketValue {
        match self {
            Self::AvailableCoins { .. }
            | Self::CoinAveragePrice { .. }
            | Self::CoinAveragePriceHistorical { .. }
            | Self::CoinAveragePriceCandles { .. }
            | Self::CoinAveragePriceCandlesHistorical { .. } => MarketValue::PairAveragePrice,
            Self::CoinExchangePrice { .. } => MarketValue::PairExchangePrice,
            Self::CoinExchangeVolume { .. } => MarketValue::PairExchangeVolume,
            Self::IndexPrice { .. }
            | Self::IndexPriceCandles { .. }
            | Self::IndexPriceCandlesHistorical { .. }
            | Self::IndexPriceHistorical { .. } => MarketValue::IndexPrice,
            Self::Unsubscribe { .. } => unreachable!(),
        }
    }
}

impl FromStr for WsChannelName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "available_coins" => Ok(Self::AvailableCoins),
            "index_price" => Ok(Self::IndexPrice),
            "index_price_candles" => Ok(Self::IndexPriceCandles),
            "coin_average_price" => Ok(Self::CoinAveragePrice),
            "coin_average_price_candles" => Ok(Self::CoinAveragePriceCandles),
            "coin_exchange_price" => Ok(Self::CoinExchangePrice),
            "coin_exchange_volume" => Ok(Self::CoinExchangeVolume),
            "index_price_historical" => Ok(Self::IndexPriceHistorical),
            "index_price_candles_historical" => Ok(Self::IndexPriceCandlesHistorical),
            "coin_average_price_historical" => Ok(Self::CoinAveragePriceHistorical),
            "coin_average_price_candles_historical" => Ok(Self::CoinAveragePriceCandlesHistorical),
            _ => Err(()),
        }
    }
}

impl ToString for WsChannelName {
    fn to_string(&self) -> String {
        match self {
            Self::AvailableCoins { .. } => "available_coins".to_string(),
            Self::IndexPrice { .. } => "index_price".to_string(),
            Self::IndexPriceCandles { .. } => "index_price_candles".to_string(),
            Self::CoinAveragePrice { .. } => "coin_average_price".to_string(),
            Self::CoinAveragePriceCandles { .. } => "coin_average_price_candles".to_string(),
            Self::CoinExchangePrice { .. } => "coin_exchange_price".to_string(),
            Self::CoinExchangeVolume { .. } => "coin_exchange_volume".to_string(),
            Self::IndexPriceHistorical { .. } => "index_price_historical".to_string(),
            Self::IndexPriceCandlesHistorical { .. } => {
                "index_price_candles_historical".to_string()
            }
            Self::CoinAveragePriceHistorical { .. } => "coin_average_price_historical".to_string(),
            Self::CoinAveragePriceCandlesHistorical { .. } => {
                "coin_average_price_candles_historical".to_string()
            }
            Self::Unsubscribe => unreachable!(),
        }
    }
}
