use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{future, pin_mut, StreamExt};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{sleep, Duration};

pub struct WsClientForTesting {
    uri: String,
    messages: Vec<String>,
    incoming_msg_tx: UnboundedSender<String>,
}

impl WsClientForTesting {
    pub fn new(
        uri: String,
        messages: Vec<String>,
        incoming_msg_tx: UnboundedSender<String>,
    ) -> Self {
        Self {
            uri,
            messages,
            incoming_msg_tx,
        }
    }

    pub async fn run(self) {
        match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => {
                let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();

                for message in self.messages {
                    stdin_tx.unbounded_send(Message::text(message)).unwrap();

                    // To prevent DDoS attack on a server
                    sleep(Duration::from_millis(100)).await;
                }

                let (write, read) = ws_stream.split();

                let stdin_to_ws = stdin_rx.map(Ok).forward(write);
                let ws_to_stdout = read.for_each(|message| async {
                    if let Ok(message) = message {
                        let message = message.into_text().unwrap();

                        let _ = self.incoming_msg_tx.send(message);
                    }
                });

                pin_mut!(stdin_to_ws, ws_to_stdout);
                future::select(stdin_to_ws, ws_to_stdout).await;
            }
            Err(e) => error!(
                "Websocket: Failed to connect. Url: {}, error: {:?}",
                self.uri, e
            ),
        }
    }
}
