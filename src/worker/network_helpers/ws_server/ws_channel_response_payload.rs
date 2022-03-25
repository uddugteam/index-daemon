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
