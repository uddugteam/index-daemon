use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum JsonRpcId {
    Int(i64),
    Str(String),
}

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    pub id: JsonRpcId,
    pub method: WsChannelName,
    pub params: serde_json::Value,
}
