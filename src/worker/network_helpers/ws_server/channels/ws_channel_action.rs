use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::channels::ws_channel_unsubscribe::WsChannelUnsubscribe;

#[derive(Debug, Clone)]
pub enum WsChannelAction {
    Subscribe(WsChannelSubscriptionRequest),
    Unsubscribe(WsChannelUnsubscribe),
}
