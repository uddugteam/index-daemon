use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{future, pin_mut, StreamExt};
use std::thread;
use std::time;

pub struct WsClientForTesting<F>
where
    F: Fn(String),
{
    uri: String,
    messages: Vec<String>,
    callback: F,
}

impl<F> WsClientForTesting<F>
where
    F: Fn(String),
{
    pub fn new(uri: String, messages: Vec<String>, callback: F) -> Self {
        Self {
            uri,
            messages,
            callback,
        }
    }

    pub fn start(self) {
        let _ = task::block_on(Self::run(self));
    }

    async fn run(self) {
        match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => {
                let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();

                for message in self.messages {
                    info!("WsClientForTesting. Message sent: {}", message);

                    stdin_tx.unbounded_send(Message::text(message)).unwrap();

                    // To prevent DDoS attack on a server
                    thread::sleep(time::Duration::from_millis(100));
                }

                let (write, read) = ws_stream.split();

                let stdin_to_ws = stdin_rx.map(Ok).forward(write);
                let ws_to_stdout = read.for_each(|message| async {
                    if let Ok(message) = message {
                        let message = message.into_text().unwrap();

                        (self.callback)(message);
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
