use crate::worker::helper_functions::add_jsonrpc_version_and_method;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use async_tungstenite::tungstenite::protocol::Message;
use futures::channel::mpsc::{TrySendError, UnboundedSender};

type Tx = UnboundedSender<Message>;

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
