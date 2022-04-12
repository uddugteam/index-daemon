use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::hepler_functions::ws_send_response;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use async_tungstenite::tungstenite::protocol::Message;
use chrono::{DateTime, Utc, MAX_DATETIME, MIN_DATETIME};
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use tokio::time::{sleep, Duration};

type Tx = UnboundedSender<Message>;

#[derive(Clone)]
pub struct WsChannelResponseSender {
    broadcast_recipient: Tx,
    pub request: WsChannelSubscriptionRequest,
    last_send_timestamp: DateTime<Utc>,
}

impl WsChannelResponseSender {
    pub fn new(broadcast_recipient: Tx, request: WsChannelSubscriptionRequest) -> Self {
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
            message: "Successfully subscribed.".to_string(),
        };
        let response = WsChannelResponse {
            id: Some(self.request.get_id()),
            result: response_payload,
        };

        self.send_inner(response)
    }

    pub async fn send(
        &mut self,
        response: WsChannelResponse,
    ) -> Option<Result<(), TrySendError<Message>>> {
        let frequency_ms = self.request.get_frequency_ms();

        let timestamp = if let Some(timestamp) = response.result.get_timestamp() {
            timestamp
        } else {
            // We sleep because if we send response immediately, there will bw an opportunity to DDoS our server
            sleep(Duration::from_millis(frequency_ms)).await;

            MAX_DATETIME
        };

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
