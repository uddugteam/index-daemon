use crate::worker::helper_functions::add_jsonrpc_version;
use crate::worker::network_helpers::ws_server::jsonrpc_messages::{JsonRpcId, JsonRpcResponse};
use crate::worker::network_helpers::ws_server::ws_channel_request::WsChannelRequest;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use async_tungstenite::tungstenite::protocol::Message;
use chrono::{DateTime, Utc, MIN_DATETIME};
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use std::cmp;

type Tx = UnboundedSender<Message>;

#[derive(Clone)]
pub struct WsChannelResponseSender {
    broadcast_recipient: Tx,
    pub request: WsChannelRequest,
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

    fn send_inner(&mut self, response: WsChannelResponse) -> Result<(), TrySendError<Message>> {
        let response_payload = response.get_payload();
        let response_payload_str = serde_json::to_string(&response_payload).unwrap();

        let response = JsonRpcResponse::Succ {
            id: response.get_id(),
            result: serde_json::from_str(&response_payload_str).unwrap(),
        };
        let mut response = serde_json::to_string(&response).unwrap();
        add_jsonrpc_version(&mut response);

        let response = Message::from(response);

        self.broadcast_recipient.unbounded_send(response)
    }

    pub fn send_succ_sub_notif(
        &mut self,
        id: Option<JsonRpcId>,
    ) -> Result<(), TrySendError<Message>> {
        let response_payload =
            WsChannelResponsePayload::SuccSub("Successfully subscribed.".to_string());
        let response = response_payload.make_response(id);

        self.send_inner(response)
    }

    pub fn send(
        &mut self,
        response: WsChannelResponse,
    ) -> Option<Result<(), TrySendError<Message>>> {
        let timestamp = response.get_payload().get_timestamp();
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
