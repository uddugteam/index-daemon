use crate::worker::network_helpers::ws_server::holders::helper_functions::{
    HolderHashMap, HolderKey,
};
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;

#[derive(Clone)]
pub struct WsChannelsHolder(HolderHashMap<WsChannels>);

impl WsChannelsHolder {
    pub fn new(ws_channels_holder: HolderHashMap<WsChannels>) -> Self {
        Self(ws_channels_holder)
    }

    pub fn contains_key(&self, key: &HolderKey) -> bool {
        self.0.contains_key(key)
    }

    pub fn add(&self, holder_key: &HolderKey, value: (String, WsChannelResponseSender)) {
        if let Some(ws_channels) = self.0.get(holder_key) {
            let (conn_id, response_sender) = value;

            ws_channels
                .write()
                .unwrap()
                .add_channel(conn_id, response_sender);
        }
    }

    pub fn remove(&self, ws_channels_key: &(String, WsChannelName)) {
        for ws_channels in self.0.values() {
            ws_channels.write().unwrap().remove_channel(ws_channels_key);
        }
    }
}
