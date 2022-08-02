use crate::config_scheme::async_from::AsyncFrom;
use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::connection_id::ConnectionId;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use async_trait::async_trait;
use async_tungstenite::tungstenite::protocol::Message;
use futures::channel::mpsc::TrySendError;
use std::collections::HashMap;

pub type CJ = (ConnectionId, JsonRpcId);

pub struct WsChannels(HashMap<CJ, WsChannelResponseSender>);

impl WsChannels {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_channels_by_method(
        &self,
        method: WsChannelName,
    ) -> HashMap<CJ, WsChannelSubscriptionRequest> {
        self.0
            .iter()
            .filter(|(_, v)| v.request.get_method() == method)
            .map(|(k, v)| (k.clone(), v.request.clone()))
            .collect()
    }

    async fn send_inner(
        sender: &mut WsChannelResponseSender,
        response_payload: WsChannelResponsePayload,
    ) -> Option<Result<(), TrySendError<Message>>> {
        let response = WsChannelResponse {
            id: Some(sender.request.get_id()),
            result: response_payload,
        };

        sender.send(response).await
    }

    pub async fn send_individual(
        &mut self,
        responses: HashMap<CJ, WsChannelResponsePayload>,
    ) -> Vec<CJ> {
        let mut keys_to_remove = Vec::new();

        for (key, response_payload) in responses {
            if let Some(sender) = self.0.get_mut(&key) {
                let send_result = Self::send_inner(sender, response_payload.clone()).await;

                if let Some(send_result) = send_result {
                    if send_result.is_err() {
                        // Send msg error. The client is likely disconnected. We stop sending him messages.

                        keys_to_remove.push(key.clone());
                    }
                } else {
                    // Message wasn't sent because of frequency_ms (not enough time has passed since last dispatch)
                }
            }
        }

        for key in &keys_to_remove {
            self.0.remove(key);
        }

        keys_to_remove
    }

    pub fn add_channel(&mut self, conn_id: ConnectionId, sender: WsChannelResponseSender) {
        let sub_id = sender.request.get_id();

        self.0.insert((conn_id, sub_id), sender);
    }

    pub fn remove_channel(&mut self, key: &CJ) {
        self.0.remove(key);
    }
}

#[async_trait]
impl AsyncFrom<(ConfigScheme, Option<RepositoryForF64ByTimestamp>)> for WsChannels {
    async fn from(
        (_config, repository): (ConfigScheme, Option<RepositoryForF64ByTimestamp>),
    ) -> Self {
        Self::new()
    }
}
