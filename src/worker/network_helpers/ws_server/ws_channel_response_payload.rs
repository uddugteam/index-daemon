use crate::worker::network_helpers::ws_server::coin_average_price_historical_snapshot::CoinAveragePriceHistoricalSnapshots;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
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
    CoinAveragePriceHistorical {
        coin: String,
        values: CoinAveragePriceHistoricalSnapshots,
    },
    CoinAveragePriceCandles {
        coin: String,
        open: f64,
        close: f64,
        min: f64,
        max: f64,
        avg: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
}

impl WsChannelResponsePayload {
    pub fn get_method(&self) -> Option<WsChannelName> {
        match self {
            Self::CoinAveragePrice { .. } => Some(WsChannelName::CoinAveragePrice),
            Self::CoinExchangePrice { .. } => Some(WsChannelName::CoinExchangePrice),
            Self::CoinExchangeVolume { .. } => Some(WsChannelName::CoinExchangeVolume),
            Self::CoinAveragePriceHistorical { .. } => {
                Some(WsChannelName::CoinAveragePriceHistorical)
            }
            Self::CoinAveragePriceCandles { .. } => Some(WsChannelName::CoinAveragePriceCandles),
            Self::SuccSub { method, .. } => Some(*method),
            Self::Err { method, .. } => *method,
        }
    }

    pub fn get_coin(&self) -> String {
        match self {
            Self::CoinAveragePrice { coin, .. }
            | Self::CoinExchangePrice { coin, .. }
            | Self::CoinExchangeVolume { coin, .. }
            | Self::CoinAveragePriceHistorical { coin, .. }
            | Self::CoinAveragePriceCandles { coin, .. } => coin.to_string(),
            Self::SuccSub { .. } | Self::Err { .. } => {
                unreachable!()
            }
        }
    }

    pub fn get_timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::CoinAveragePrice { timestamp, .. }
            | Self::CoinExchangePrice { timestamp, .. }
            | Self::CoinExchangeVolume { timestamp, .. }
            | Self::CoinAveragePriceCandles { timestamp, .. } => *timestamp,
            Self::CoinAveragePriceHistorical { .. } | Self::SuccSub { .. } | Self::Err { .. } => {
                unreachable!()
            }
        }
    }
}
