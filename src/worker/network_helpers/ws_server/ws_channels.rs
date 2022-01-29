use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use std::collections::HashMap;

pub struct WsChannels(HashMap<(String, String), WsChannelResponseSender>);

impl WsChannels {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn send(&mut self, response_payload: WsChannelResponsePayload) {
        let coin = response_payload.get_coin();

        let senders: HashMap<&(String, String), &mut WsChannelResponseSender> = self
            .0
            .iter_mut()
            .filter(|(_, v)| v.request.get_coins().contains(&coin))
            .collect();

        let mut keys_to_remove = Vec::new();

        for (key, sender) in senders {
            let response = response_payload
                .clone()
                .make_response(sender.request.get_id());

            let send_msg_result = sender.send(response);

            if let Some(send_msg_result) = send_msg_result {
                if send_msg_result.is_err() {
                    // Send msg error. The client is likely disconnected. We stop sending him messages.

                    keys_to_remove.push(key.clone());
                }
            } else {
                // Message wasn't sent because of frequency_ms (not enough time has passed since last dispatch)
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

    pub fn remove_channel(&mut self, key: &(String, String)) {
        self.0.remove(key);
    }
}

#[cfg(test)]
pub mod test {
    use crate::worker::network_helpers::ws_server::jsonrpc_messages::JsonRpcId;
    use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;

    pub fn check_subscriptions(
        ws_channels: &WsChannels,
        subscriptions: &Vec<(String, String, Vec<String>)>,
    ) {
        assert_eq!(subscriptions.len(), ws_channels.0.len());

        for (sub_id, method, coins) in subscriptions {
            let keys: Vec<&(String, String)> = ws_channels
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
            assert_eq!(channel.request.get_coins(), coins);
        }
    }
}
