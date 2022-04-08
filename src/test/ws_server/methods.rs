use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::helper_functions::{
    check_incoming_messages, start_application, ws_connect_and_send,
};
use crate::test::ws_server::request::{Requests, RequestsUnzipped};
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_request::WsRequest;
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[ignore]
#[serial]
fn test_request_methods_together() {
    let port = 9100;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let methods = WsChannelName::get_all_methods();

    let RequestsUnzipped {
        requests,
        params_vec: _,
        expecteds,
    } = Requests::make_all(&config, &methods, false, None).unzip();

    let (_rx, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
        start_application(config);

    let expecteds = expecteds
        .into_iter()
        .map(|v| v.unwrap())
        .map(|v| match v {
            WsRequest::Method(v) => (v.get_id(), v.get_method()),
            _ => unreachable!(),
        })
        .collect();

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);
    let (hash_map, no_id) = check_incoming_messages(incoming_msg_rx, &expecteds);

    if !no_id.is_empty() {
        panic!("Got responses with no id: {:#?}", no_id);
    }

    for (id, method) in expecteds {
        if let Some(res) = hash_map.get(&id) {
            if res.is_err() {
                panic!("Expected Ok. Got: {:#?}", res);
            }
        } else {
            panic!("No response received for method: {:#?}", method);
        }
    }
}

#[test]
#[ignore]
#[serial]
fn test_request_methods_together_with_errors() {
    let port = 9200;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let methods = WsChannelName::get_all_methods();

    let RequestsUnzipped {
        requests,
        params_vec,
        expecteds,
    } = Requests::make_all(&config, &methods, true, None).unzip();

    let (_rx, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
        start_application(config);

    let expecteds_errors: HashMap<_, _> = params_vec
        .iter()
        .zip(expecteds)
        .map(|v| (v.0.id.clone(), v.1.unwrap_err()))
        .collect();

    let expecteds = params_vec.into_iter().map(|v| (v.id, v.channel)).collect();

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);
    let (hash_map, no_id) = check_incoming_messages(incoming_msg_rx, &expecteds);

    let mut expecteds_with_no_ids = Vec::new();
    for (id, method) in expecteds {
        if let Some(res) = hash_map.get(&id) {
            if res.is_ok() {
                panic!("Expected Err. Got: {:#?}", res);
            }
        } else {
            let error_type = expecteds_errors.get(&id).unwrap();

            expecteds_with_no_ids.push((method, error_type));
        }
    }

    if expecteds_with_no_ids.len() > no_id.len() {
        panic!(
            "No response received for some of requests. Expected count: {}. Received count: {}. Expecteds: {:#?}.",
            expecteds_with_no_ids.len(), no_id.len(), expecteds_with_no_ids,
        );
    }
}
