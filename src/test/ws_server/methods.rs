use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::helper_functions::{
    check_incoming_messages, start_application, ws_connect_and_send,
};
use crate::test::ws_server::request::{Requests, RequestsUnzipped};
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_request::WsRequest;
use serial_test::serial;

#[test]
#[serial]
fn test_request_methods_together() {
    let mut port = 9100;
    let mut config = ConfigScheme::default();
    let methods = WsChannelName::get_all_methods();

    let RequestsUnzipped {
        requests,
        params_vec: _,
        expecteds,
    } = Requests::make_all(&config, &methods, false, None).unzip();

    config.service.ws_addr = format!("127.0.0.1:{}", port);
    port += 1;

    let (_rx, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
        start_application(config);

    let expecteds: Vec<WsChannelName> = expecteds
        .into_iter()
        .map(|v| v.unwrap())
        .map(|v| match v {
            WsRequest::Method(v) => v.get_method(),
            _ => unreachable!(),
        })
        .collect();

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);
    let res = check_incoming_messages(incoming_msg_rx, &expecteds);
    if res.is_err() {
        panic!("Expected Ok. Got: {:?}", res);
    }
}
