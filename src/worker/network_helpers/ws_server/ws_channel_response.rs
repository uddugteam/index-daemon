use crate::worker::network_helpers::ws_server::json_rpc_messages::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum WsChannelResponse {
    SuccSub {
        id: Option<JsonRpcId>,
        payload: WsChannelResponsePayload,
    },
    CoinAveragePrice {
        id: Option<JsonRpcId>,
        payload: WsChannelResponsePayload,
    },
    CoinExchangePrice {
        id: Option<JsonRpcId>,
        payload: WsChannelResponsePayload,
    },
}

impl WsChannelResponse {
    pub fn get_payload(&self) -> WsChannelResponsePayload {
        match self {
            WsChannelResponse::SuccSub { payload, .. }
            | WsChannelResponse::CoinAveragePrice { payload, .. }
            | WsChannelResponse::CoinExchangePrice { payload, .. } => payload.clone(),
        }
    }

    pub fn get_id(&self) -> Option<JsonRpcId> {
        match self {
            WsChannelResponse::SuccSub { id, .. }
            | WsChannelResponse::CoinAveragePrice { id, .. }
            | WsChannelResponse::CoinExchangePrice { id, .. } => id.clone(),
        }
    }
}
