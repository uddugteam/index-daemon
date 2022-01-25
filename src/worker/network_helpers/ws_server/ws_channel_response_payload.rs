use crate::worker::network_helpers::ws_server::json_rpc_messages::JsonRpcId;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;

use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum WsChannelResponsePayload {
    SuccSub(String),
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
    pub fn make_response(self, id: Option<JsonRpcId>) -> WsChannelResponse {
        match &self {
            WsChannelResponsePayload::CoinAveragePrice { .. } => {
                WsChannelResponse::CoinAveragePrice { id, payload: self }
            }
            WsChannelResponsePayload::CoinExchangePrice { .. } => {
                WsChannelResponse::CoinExchangePrice { id, payload: self }
            }
            WsChannelResponsePayload::CoinExchangeVolume { .. } => {
                WsChannelResponse::CoinExchangeVolume { id, payload: self }
            }
            WsChannelResponsePayload::SuccSub(..) => {
                WsChannelResponse::SuccSub { id, payload: self }
            }
        }
    }

    pub fn get_coin(&self) -> String {
        match self {
            WsChannelResponsePayload::CoinAveragePrice { coin, .. }
            | WsChannelResponsePayload::CoinExchangePrice { coin, .. }
            | WsChannelResponsePayload::CoinExchangeVolume { coin, .. } => coin.to_string(),
            WsChannelResponsePayload::SuccSub(..) => panic!("Wrong response."),
        }
    }

    pub fn get_timestamp(&self) -> DateTime<Utc> {
        match self {
            WsChannelResponsePayload::CoinAveragePrice { timestamp, .. }
            | WsChannelResponsePayload::CoinExchangePrice { timestamp, .. }
            | WsChannelResponsePayload::CoinExchangeVolume { timestamp, .. } => *timestamp,
            WsChannelResponsePayload::SuccSub(..) => panic!("Wrong response."),
        }
    }
}
