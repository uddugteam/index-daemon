use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use async_tungstenite::tungstenite::protocol::Message;
use futures::channel::mpsc::{TrySendError, UnboundedSender};

type Tx = UnboundedSender<Message>;

pub fn add_jsonrpc_version_and_method(response: &mut String, method: Option<String>) {
    let mut value: serde_json::Value = serde_json::from_str(response).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert(
        "jsonrpc".to_string(),
        serde_json::Value::from("2.0".to_string()),
    );

    if let Some(method) = method {
        let object = object.get_mut("result").unwrap().as_object_mut().unwrap();
        object.insert("method".to_string(), serde_json::Value::from(method));
    }

    *response = value.to_string();
}

pub fn ws_send_response(
    broadcast_recipient: &Tx,
    response: WsChannelResponse,
    method: Option<String>,
) -> Result<(), TrySendError<Message>> {
    let mut response = serde_json::to_string(&response).unwrap();
    add_jsonrpc_version_and_method(&mut response, method);

    let response = Message::from(response);

    broadcast_recipient.unbounded_send(response)
}
