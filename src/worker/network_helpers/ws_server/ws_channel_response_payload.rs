use crate::worker::network_helpers::ws_server::candles::{Candle, Candles};
use crate::worker::network_helpers::ws_server::f64_snapshot::F64Snapshots;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Clone)]
pub struct CoinPrice {
    pub coin: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum WsChannelResponsePayload {
    SuccSub {
        method: WsChannelName,
        message: String,
    },
    Err {
        method: Option<WsChannelName>,
        code: i64,
        message: String,
    },
    AvailableCoins {
        coins: Vec<CoinPrice>,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
    IndexPrice {
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
    CoinAveragePrice {
        coin: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
    CoinExchangePrice {
        coin: String,
        exchange: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
    CoinExchangeVolume {
        coin: String,
        exchange: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
    IndexPriceHistorical {
        values: F64Snapshots,
    },
    CoinAveragePriceHistorical {
        coin: String,
        values: F64Snapshots,
    },
    IndexPriceCandles {
        value: Candle,
    },
    IndexPriceCandlesHistorical {
        values: Candles,
    },
    CoinAveragePriceCandles {
        coin: String,
        value: Candle,
    },
    CoinAveragePriceCandlesHistorical {
        coin: String,
        values: Candles,
    },
}

impl WsChannelResponsePayload {
    pub fn get_method(&self) -> Option<WsChannelName> {
        match self {
            Self::AvailableCoins { .. } => Some(WsChannelName::AvailableCoins),
            Self::IndexPrice { .. } => Some(WsChannelName::IndexPrice),
            Self::CoinAveragePrice { .. } => Some(WsChannelName::CoinAveragePrice),
            Self::CoinExchangePrice { .. } => Some(WsChannelName::CoinExchangePrice),
            Self::CoinExchangeVolume { .. } => Some(WsChannelName::CoinExchangeVolume),
            Self::IndexPriceHistorical { .. } => Some(WsChannelName::IndexPriceHistorical),
            Self::CoinAveragePriceHistorical { .. } => {
                Some(WsChannelName::CoinAveragePriceHistorical)
            }
            Self::IndexPriceCandles { .. } => Some(WsChannelName::IndexPriceCandles),
            Self::IndexPriceCandlesHistorical { .. } => {
                Some(WsChannelName::IndexPriceCandlesHistorical)
            }
            Self::CoinAveragePriceCandles { .. } => Some(WsChannelName::CoinAveragePriceCandles),
            Self::CoinAveragePriceCandlesHistorical { .. } => {
                Some(WsChannelName::CoinAveragePriceCandlesHistorical)
            }
            Self::SuccSub { method, .. } => Some(*method),
            Self::Err { method, .. } => *method,
        }
    }

    pub fn get_coin(&self) -> Option<String> {
        match self {
            Self::CoinAveragePrice { coin, .. }
            | Self::CoinExchangePrice { coin, .. }
            | Self::CoinExchangeVolume { coin, .. }
            | Self::CoinAveragePriceHistorical { coin, .. }
            | Self::CoinAveragePriceCandles { coin, .. }
            | Self::CoinAveragePriceCandlesHistorical { coin, .. } => Some(coin.to_string()),
            Self::AvailableCoins { .. }
            | Self::IndexPrice { .. }
            | Self::IndexPriceHistorical { .. }
            | Self::IndexPriceCandles { .. }
            | Self::IndexPriceCandlesHistorical { .. }
            | Self::SuccSub { .. }
            | Self::Err { .. } => None,
        }
    }

    pub fn get_timestamp(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::AvailableCoins { timestamp, .. }
            | Self::IndexPrice { timestamp, .. }
            | Self::CoinAveragePrice { timestamp, .. }
            | Self::CoinExchangePrice { timestamp, .. }
            | Self::CoinExchangeVolume { timestamp, .. } => Some(*timestamp),
            Self::IndexPriceCandles { value, .. } | Self::CoinAveragePriceCandles { value, .. } => {
                Some(value.timestamp)
            }
            Self::IndexPriceHistorical { .. }
            | Self::IndexPriceCandlesHistorical { .. }
            | Self::CoinAveragePriceHistorical { .. }
            | Self::CoinAveragePriceCandlesHistorical { .. }
            | Self::SuccSub { .. }
            | Self::Err { .. } => None,
        }
    }
}
