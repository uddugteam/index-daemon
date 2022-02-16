use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::hepler_functions::ws_send_response;
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
    pub request: WsChannelSubscriptionRequest,
    last_send_timestamp: DateTime<Utc>,
}

impl WsChannelResponseSender {
    pub fn new(
        broadcast_recipient: Tx,
        mut request: WsChannelSubscriptionRequest,
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

    fn send_inner(&self, response: WsChannelResponse) -> Result<(), TrySendError<Message>> {
        ws_send_response(
            &self.broadcast_recipient,
            response,
            Some(self.request.get_method()),
        )
    }

    pub fn send_succ_sub_notif(&self) -> Result<(), TrySendError<Message>> {
        let response_payload = WsChannelResponsePayload::SuccSub {
            method: self.request.get_method(),
            message: "Successfully subscribed.".to_string(),
        };
        let response = WsChannelResponse {
            id: self.request.get_id(),
            result: response_payload,
        };

        self.send_inner(response)
    }

    pub fn send(
        &mut self,
        response: WsChannelResponse,
    ) -> Option<Result<(), TrySendError<Message>>> {
        let timestamp = response.result.get_timestamp();
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
