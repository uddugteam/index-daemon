use crate::config_scheme::config_scheme::ConfigScheme;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::connection_id::ConnectionId;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use std::collections::HashMap;

pub struct WsChannels(HashMap<(ConnectionId, JsonRpcId), WsChannelResponseSender>);

impl WsChannels {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_channels_by_method(
        &self,
        method: WsChannelName,
    ) -> HashMap<&(ConnectionId, JsonRpcId), &WsChannelSubscriptionRequest> {
        self.0
            .iter()
            .filter(|(_, v)| v.request.get_method() == method)
            .map(|(k, v)| (k, &v.request))
            .collect()
    }

    fn send_inner(
        sender: &mut WsChannelResponseSender,
        response_payload: WsChannelResponsePayload,
    ) -> Result<(), ()> {
        let response = WsChannelResponse {
            id: Some(sender.request.get_id()),
            result: response_payload,
        };

        if let Some(send_msg_result) = sender.send(response) {
            if send_msg_result.is_err() {
                // Send msg error. The client is likely disconnected. We stop sending him messages.

                Err(())
            } else {
                Ok(())
            }
        } else {
            // Message wasn't sent because of frequency_ms (not enough time has passed since last dispatch)

            Ok(())
        }
    }

    pub fn send_individual(
        &mut self,
        responses: HashMap<(ConnectionId, JsonRpcId), WsChannelResponsePayload>,
    ) {
        let mut keys_to_remove = Vec::new();

        for (key, response_payload) in responses {
            if let Some(sender) = self.0.get_mut(&key) {
                let send_result = Self::send_inner(sender, response_payload.clone());

                if send_result.is_err() {
                    // Send msg error. The client is likely disconnected. We stop sending him messages.

                    keys_to_remove.push(key.clone());
                }
            }
        }

        for key in keys_to_remove {
            self.0.remove(&key);
        }
    }

    pub fn add_channel(&mut self, conn_id: ConnectionId, channel: WsChannelResponseSender) {
        let sub_id = channel.request.get_id();

        if channel.send_succ_sub_notif().is_ok() {
            self.0.insert((conn_id, sub_id), channel);
        } else {
            // Send msg error. The client is likely disconnected. Thus, we don't even establish subscription.
        }
    }

    pub fn remove_channel(&mut self, key: &(ConnectionId, JsonRpcId)) {
        self.0.remove(key);
    }
}

impl From<ConfigScheme> for WsChannels {
    fn from(_config: ConfigScheme) -> Self {
        Self::new()
    }
}
