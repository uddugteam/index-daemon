use crate::worker::network_helpers::ws_server::coin_average_price_historical_snapshot::CoinAveragePriceHistoricalSnapshots;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum WsChannelResponsePayload {
    SuccSub {
        method: String,
        message: String,
    },
    Err {
        method: String,
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
}

impl WsChannelResponsePayload {
    pub fn get_method(&self) -> String {
        match self {
            Self::CoinAveragePrice { .. } => "coin_average_price".to_string(),
            Self::CoinExchangePrice { .. } => "coin_exchange_price".to_string(),
            Self::CoinExchangeVolume { .. } => "coin_exchange_volume".to_string(),
            Self::CoinAveragePriceHistorical { .. } => "coin_average_price_historical".to_string(),
            Self::SuccSub { method, .. } | Self::Err { method, .. } => method.to_string(),
        }
    }

    pub fn get_coin(&self) -> String {
        match self {
            Self::CoinAveragePrice { coin, .. }
            | Self::CoinExchangePrice { coin, .. }
            | Self::CoinExchangeVolume { coin, .. }
            | Self::CoinAveragePriceHistorical { coin, .. } => coin.to_string(),
            Self::SuccSub { .. } | Self::Err { .. } => {
                unreachable!()
            }
        }
    }

    pub fn get_timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::CoinAveragePrice { timestamp, .. }
            | Self::CoinExchangePrice { timestamp, .. }
            | Self::CoinExchangeVolume { timestamp, .. } => *timestamp,
            Self::CoinAveragePriceHistorical { .. } | Self::SuccSub { .. } | Self::Err { .. } => {
                unreachable!()
            }
        }
    }
}
