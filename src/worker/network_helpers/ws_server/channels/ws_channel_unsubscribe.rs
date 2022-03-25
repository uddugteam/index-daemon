use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone)]
pub struct WsChannelUnsubscribe {
    pub id: JsonRpcId,
}

impl From<WsChannelSubscriptionRequest> for WsChannelUnsubscribe {
    fn from(request: WsChannelSubscriptionRequest) -> Self {
        Self {
            id: request.get_id(),
        }
    }
}
