use chrono::{DateTime, Utc};

use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;

#[derive(Debug, Serialize)]
pub enum WsChannelResponse {
    CoinAveragePrice {
        coin: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
}

impl WsChannelResponse {
    pub fn get_timestamp(&self) -> DateTime<Utc> {
        let timestamp = match self {
            WsChannelResponse::CoinAveragePrice { timestamp, .. } => *timestamp,
        };

        timestamp
    }
}
