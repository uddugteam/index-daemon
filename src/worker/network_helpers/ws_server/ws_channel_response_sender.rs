use crate::worker::network_helpers::ws_server::ws_channel_request::WsChannelRequest;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_server::{WsServer, JSONRPC_RESPONSE_SUCCESS};
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

    pub fn get_coins(&self) -> &Vec<String> {
        self.request.get_coins()
    }

    fn send_inner(&mut self, response_value: String) -> Result<(), TrySendError<Message>> {
        let mut response = JSONRPC_RESPONSE_SUCCESS.to_string();
        WsServer::fill_response_success(&mut response, response_value);
        let response = Message::from(response);

        self.broadcast_recipient.unbounded_send(response)
    }

    pub fn send_succ_sub_notif(&mut self) -> Result<(), TrySendError<Message>> {
        let response_value = "Successfully subscribed.".to_string();

        self.send_inner(response_value)
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

            let response_value = serde_json::to_string(&response).unwrap();
            let send_msg_result = self.send_inner(response_value);

            Some(send_msg_result)
        } else {
            // Too early
            None
        }
    }
}
