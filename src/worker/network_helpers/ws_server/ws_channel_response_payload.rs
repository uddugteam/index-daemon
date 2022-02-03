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
}

impl WsChannelResponsePayload {
    pub fn get_coin(&self) -> String {
        match self {
            Self::CoinAveragePrice { coin, .. }
            | Self::CoinExchangePrice { coin, .. }
            | Self::CoinExchangeVolume { coin, .. } => coin.to_string(),
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
            Self::SuccSub { .. } | Self::Err { .. } => {
                unreachable!()
            }
        }
    }
}
