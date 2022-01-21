use crate::worker::network_helpers::ws_server::ws_server::{WsServer, JSONRPC_RESPONSE_SUCCESS};
use async_tungstenite::tungstenite::protocol::Message;
use chrono::{DateTime, Utc, MIN_DATETIME};
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use serde_json::json;

type Tx = UnboundedSender<Message>;

pub struct CoinAveragePriceChannelSender {
    broadcast_recipient: Tx,
    pub coins: Vec<String>,
    frequency_ms: u64,
    last_send_timestamp: DateTime<Utc>,
}

impl CoinAveragePriceChannelSender {
    pub fn new(broadcast_recipient: Tx, coins: Vec<String>, frequency_ms: u64) -> Self {
        Self {
            broadcast_recipient,
            coins,
            frequency_ms,
            last_send_timestamp: MIN_DATETIME,
        }
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
        coin: String,
        value: f64,
        timestamp: DateTime<Utc>,
    ) -> Option<Result<(), TrySendError<Message>>> {
        if (timestamp - self.last_send_timestamp).num_milliseconds() as u64 > self.frequency_ms {
            // Enough time passed
            self.last_send_timestamp = Utc::now();

            let response_value = json!({
                "coin": coin,
                "value": value,
                "timestamp": timestamp.timestamp()
            })
            .to_string();

            let send_msg_result = self.send_inner(response_value);

            Some(send_msg_result)
        } else {
            // Too early
            None
        }
    }
}
