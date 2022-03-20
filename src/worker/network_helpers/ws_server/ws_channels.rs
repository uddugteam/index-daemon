use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use std::collections::HashMap;

pub struct WsChannels(HashMap<(String, WsChannelName), WsChannelResponseSender>);

impl WsChannels {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_channels_by_method(
        &self,
        method: WsChannelName,
    ) -> HashMap<&(String, WsChannelName), &WsChannelSubscriptionRequest> {
        self.0
            .iter()
            .filter(|(k, _)| k.1 == method)
            .map(|(k, v)| (k, &v.request))
            .collect()
    }

    fn send_inner(
        sender: &mut WsChannelResponseSender,
        response_payload: WsChannelResponsePayload,
    ) -> Result<(), ()> {
        let response = WsChannelResponse {
            id: sender.request.get_id(),
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
        responses: HashMap<(String, WsChannelName), WsChannelResponsePayload>,
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

    pub fn send_general(&mut self, response_payload: WsChannelResponsePayload) {
        let senders: HashMap<&(String, WsChannelName), &mut WsChannelResponseSender> =
            match response_payload.get_coin() {
                Some(coin) => self
                    .0
                    .iter_mut()
                    .filter(|(_, v)| v.request.get_coins().is_some())
                    .filter(|(_, v)| v.request.get_coins().unwrap().contains(&coin))
                    .collect(),
                None => self
                    .0
                    .iter_mut()
                    .filter(|(_, v)| v.request.get_coins().is_none())
                    .collect(),
            };

        let mut keys_to_remove = Vec::new();

        if let Some(response_method) = response_payload.get_method() {
            for (key, sender) in senders {
                if key.1 == response_method {
                    let send_result = Self::send_inner(sender, response_payload.clone());

                    if send_result.is_err() {
                        // Send msg error. The client is likely disconnected. We stop sending him messages.

                        keys_to_remove.push(key.clone());
                    }
                }
            }
        }

        for key in keys_to_remove {
            self.0.remove(&key);
        }
    }

    pub fn add_channel(&mut self, conn_id: String, channel: WsChannelResponseSender) {
        let method = channel.request.get_method();

        if channel.send_succ_sub_notif().is_ok() {
            self.0.insert((conn_id, method), channel);
        } else {
            // Send msg error. The client is likely disconnected. Thus, we don't even establish subscription.
        }
    }

    pub fn remove_channel(&mut self, key: &(String, WsChannelName)) {
        self.0.remove(key);
    }
}

#[cfg(test)]
pub mod test {
    use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
    use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
    use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;

    pub fn check_subscriptions(
        ws_channels: &WsChannels,
        subscriptions: &[(String, WsChannelName, Vec<String>)],
    ) {
        assert_eq!(subscriptions.len(), ws_channels.0.len());

        for (sub_id, method, coins) in subscriptions {
            let keys: Vec<&(String, WsChannelName)> = ws_channels
                .0
                .keys()
                .filter(|(_conn_id, method_inner)| method_inner == method)
                .collect();
            assert_eq!(keys.len(), 1);

            let channel = ws_channels.0.get(keys[0]).unwrap();

            if let Some(JsonRpcId::Str(real_sub_id)) = channel.request.get_id() {
                assert_eq!(&real_sub_id, sub_id);
            } else {
                panic!("Wrong request id.");
            }

            assert_eq!(&channel.request.get_method(), method);
            assert_eq!(channel.request.get_coins(), Some(coins.as_slice()));
        }
    }
}
