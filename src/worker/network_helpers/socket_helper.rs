use std::borrow::Borrow;

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
        // println!("called SocketHelper::start()");
        // println!("self.uri: {}", self.uri);
        // println!("self.on_open_msg: {}", self.on_open_msg);

        task::block_on(run(self));
    }
}

async fn run<F>(socket_helper: SocketHelper<F>)
where
    F: Fn(String, String),
{
    let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();

    // TODO: Remove panic
    let (ws_stream, _) = connect_async(socket_helper.uri)
        .await
        .expect("Websocket: Failed to connect");
    // println!("WebSocket handshake has been successfully completed");

    if let Some(on_open_msg) = socket_helper.on_open_msg {
        // println!("Message sent: {}", on_open_msg);
        stdin_tx
            .unbounded_send(Message::binary(on_open_msg))
            .unwrap();
    } else {
        // println!("Message NOT sent, because it's empty");
    }

    let (write, read) = ws_stream.split();

    let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            let message = message.unwrap().into_text().unwrap();
            // println!("Got message: {:?}", mess);

            socket_helper.callback.borrow()(socket_helper.pair.clone(), message);
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}
