use crate::worker::network_helpers::ws_server::json_rpc_messages::{
    JsonRpcId, JsonRpcResponseSucc,
};
use crate::worker::network_helpers::ws_server::ws_channel_request::WsChannelRequest;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use async_tungstenite::tungstenite::protocol::Message;
use chrono::{DateTime, Utc, MIN_DATETIME};
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use std::cmp;

type Tx = UnboundedSender<Message>;

pub struct WsChannelResponseSender {
    broadcast_recipient: Tx,
    request: WsChannelRequest,
    last_send_timestamp: DateTime<Utc>,
}

impl WsChannelResponseSender {
    pub fn new(
        broadcast_recipient: Tx,
        mut request: WsChannelRequest,
        ws_answer_timeout_ms: u64,
    ) -> Self {
        let frequency_ms = request.get_frequency_ms();
        let frequency_ms = cmp::max(ws_answer_timeout_ms, frequency_ms);
        request.set_frequency_ms(frequency_ms);

        Self {
            broadcast_recipient,
            request,
            last_send_timestamp: MIN_DATETIME,
        }
    }

    pub fn get_id(&self) -> Option<JsonRpcId> {
        self.request.get_id()
    }

    pub fn get_coins(&self) -> &Vec<String> {
        self.request.get_coins()
    }

    fn add_jsonrpc_version(response: &mut String) {
        let mut value: serde_json::Value = serde_json::from_str(&response).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert(
            "jsonrpc".to_string(),
            serde_json::Value::from("2.0".to_string()),
        );

        *response = value.to_string();
    }

    fn send_inner(&mut self, response: WsChannelResponse) -> Result<(), TrySendError<Message>> {
        let response_str = serde_json::to_string(&response).unwrap();

        let response = JsonRpcResponseSucc {
            id: response.get_id(),
            result: serde_json::from_str(&response_str).unwrap(),
        };
        let mut response = serde_json::to_string(&response).unwrap();
        Self::add_jsonrpc_version(&mut response);

        let response = Message::from(response);

        self.broadcast_recipient.unbounded_send(response)
    }

    pub fn send_succ_sub_notif(&mut self) -> Result<(), TrySendError<Message>> {
        let response = WsChannelResponse::Str("Successfully subscribed.".to_string());

        self.send_inner(response)
    }

    pub fn send(
        &mut self,
        response: WsChannelResponse,
    ) -> Option<Result<(), TrySendError<Message>>> {
        let timestamp = response.get_timestamp();
        let frequency_ms = self.request.get_frequency_ms();

        if (timestamp - self.last_send_timestamp).num_milliseconds() as u64 > frequency_ms {
            // Enough time passed
            self.last_send_timestamp = Utc::now();

            let send_msg_result = self.send_inner(response);

            Some(send_msg_result)
        } else {
            // Too early
            None
        }
    }
}
