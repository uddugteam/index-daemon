use crate::worker::network_helpers::ws_server::candles::{Candle, Candles};
use crate::worker::network_helpers::ws_server::f64_snapshot::F64Snapshots;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinPrice {
    pub coin: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "method")]
#[serde(rename_all = "snake_case")]
pub enum WsChannelResponsePayload {
    SuccSub {
        message: String,
    },
    Err {
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
        percent_change_interval_sec: u64,
        percent_change: Option<f64>,
    },
    CoinAveragePrice {
        coin: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
        percent_change_interval_sec: u64,
        percent_change: Option<f64>,
    },
    CoinExchangePrice {
        coin: String,
        exchange: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
        percent_change_interval_sec: u64,
        percent_change: Option<f64>,
    },
    CoinExchangeVolume {
        coin: String,
        exchange: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
        percent_change_interval_sec: u64,
        percent_change: Option<f64>,
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
            WsChannelResponsePayload::SuccSub { .. } | WsChannelResponsePayload::Err { .. } => None,
            WsChannelResponsePayload::AvailableCoins { .. } => Some(WsChannelName::AvailableCoins),
            WsChannelResponsePayload::IndexPrice { .. } => Some(WsChannelName::IndexPrice),
            WsChannelResponsePayload::CoinAveragePrice { .. } => {
                Some(WsChannelName::CoinAveragePrice)
            }
            WsChannelResponsePayload::CoinExchangePrice { .. } => {
                Some(WsChannelName::CoinExchangePrice)
            }
            WsChannelResponsePayload::CoinExchangeVolume { .. } => {
                Some(WsChannelName::CoinExchangeVolume)
            }
            WsChannelResponsePayload::IndexPriceHistorical { .. } => {
                Some(WsChannelName::IndexPriceHistorical)
            }
            WsChannelResponsePayload::CoinAveragePriceHistorical { .. } => {
                Some(WsChannelName::CoinAveragePriceHistorical)
            }
            WsChannelResponsePayload::IndexPriceCandles { .. } => {
                Some(WsChannelName::IndexPriceCandles)
            }
            WsChannelResponsePayload::IndexPriceCandlesHistorical { .. } => {
                Some(WsChannelName::IndexPriceCandlesHistorical)
            }
            WsChannelResponsePayload::CoinAveragePriceCandles { .. } => {
                Some(WsChannelName::CoinAveragePriceCandles)
            }
            WsChannelResponsePayload::CoinAveragePriceCandlesHistorical { .. } => {
                Some(WsChannelName::CoinAveragePriceCandlesHistorical)
            }
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
