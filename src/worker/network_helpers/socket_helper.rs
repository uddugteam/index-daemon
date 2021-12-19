use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{future, pin_mut, StreamExt};

pub struct SocketHelper<F>
where
    F: Fn(String, String),
{
    uri: String,
    on_open_msg: Option<String>,
    pair: String,
    callback: F,
}

impl<F> SocketHelper<F>
where
    F: Fn(String, String),
{
    pub fn new(uri: String, on_open_msg: Option<String>, pair: String, callback: F) -> Self {
        Self {
            uri,
            on_open_msg,
            pair,
            callback,
        }
    }

    pub fn start(self) {
        task::block_on(run(self));
    }
}

async fn run<F>(socket_helper: SocketHelper<F>)
where
    F: Fn(String, String),
{
    let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();

    if let Ok((ws_stream, _)) = connect_async(&socket_helper.uri).await {
        if let Some(on_open_msg) = socket_helper.on_open_msg {
            stdin_tx
                .unbounded_send(Message::binary(on_open_msg))
                .unwrap();
        }

        let (write, read) = ws_stream.split();

        let stdin_to_ws = stdin_rx.map(Ok).forward(write);
        let ws_to_stdout = read.for_each(|message| async {
            if let Ok(message) = message {
                if let Ok(message) = message.into_text() {
                    (socket_helper.callback)(socket_helper.pair.clone(), message);
                }
            }
        });

        pin_mut!(stdin_to_ws, ws_to_stdout);
        future::select(stdin_to_ws, ws_to_stdout).await;
    } else {
        error!(
            "Websocket: Failed to connect. Url: {}, pair: {}",
            socket_helper.uri, socket_helper.pair
        );
    }
}
