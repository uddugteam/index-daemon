use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_messages::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Debug, Clone)]
pub struct WsChannelUnsubscribe {
    pub id: Option<JsonRpcId>,
    pub method: WsChannelName,
}

impl From<WsChannelSubscriptionRequest> for WsChannelUnsubscribe {
    fn from(request: WsChannelSubscriptionRequest) -> Self {
        Self {
            id: request.get_id(),
            method: request.get_method(),
        }
    }
}
