use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use async_tungstenite::tungstenite::protocol::Message;
use chrono::{DateTime, Utc, MAX_DATETIME};
use futures::channel::mpsc::{TrySendError, UnboundedSender};

type Tx = UnboundedSender<Message>;

pub fn add_jsonrpc_version_and_method(response: &mut String, method: Option<WsChannelName>) {
    let mut value: serde_json::Value = serde_json::from_str(response).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert(
        "jsonrpc".to_string(),
        serde_json::Value::from("2.0".to_string()),
    );

    if let Some(method) = method {
        let object = object.get_mut("result").unwrap().as_object_mut().unwrap();
        let method = serde_json::to_string(&method).unwrap();
        let method: serde_json::Value = serde_json::from_str(&method).unwrap();
        object.insert("method".to_string(), method);
    }

    *response = value.to_string();
}

pub fn ws_send_response(
    broadcast_recipient: &Tx,
    response: WsChannelResponse,
    method: Option<WsChannelName>,
) -> Result<(), TrySendError<Message>> {
    let mut response = serde_json::to_string(&response).unwrap();
    add_jsonrpc_version_and_method(&mut response, method);

    let response = Message::from(response);

    broadcast_recipient.unbounded_send(response)
}

pub fn thin_by_interval(
    mut values: Vec<(DateTime<Utc>, f64)>,
    interval: Interval,
) -> Vec<(DateTime<Utc>, f64)> {
    values.sort_by(|a, b| a.0.cmp(&b.0));

    let interval = interval.into_seconds();

    let mut res = Vec::new();

    if let Some(mut last_el) = values.get(0).cloned() {
        // Push dummy
        values.push((MAX_DATETIME, 0.0));

        let mut last_timestamp = values[0].0.timestamp() as u64;

        for el in values {
            if el.0.timestamp() as u64 > last_timestamp {
                res.push(last_el);
                last_timestamp += interval;
            }

            last_el = el;
        }
    }

    res
}
