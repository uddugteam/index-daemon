use crate::worker::network_helpers::ws_server::json_rpc_messages::JsonRpcId;
use chrono::{DateTime, Utc};

use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum WsChannelResponse {
    Str(String),
    CoinAveragePrice {
        id: Option<JsonRpcId>,
        coin: String,
        value: f64,
        #[serde(with = "ser_date_into_timestamp")]
        timestamp: DateTime<Utc>,
    },
}

impl WsChannelResponse {
    pub fn get_id(&self) -> Option<JsonRpcId> {
        let id = match self {
            WsChannelResponse::CoinAveragePrice { id, .. } => id.clone(),
            _ => None,
        };

        id
    }

    pub fn get_timestamp(&self) -> DateTime<Utc> {
        let timestamp = match self {
            WsChannelResponse::CoinAveragePrice { timestamp, .. } => *timestamp,
            _ => panic!("Wrong response."),
        };

        timestamp
    }
}
