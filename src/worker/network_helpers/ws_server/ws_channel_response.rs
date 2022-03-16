use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;

#[derive(Debug, Serialize, Clone)]
pub struct WsChannelResponse {
    pub id: Option<JsonRpcId>,
    pub result: WsChannelResponsePayload,
}
