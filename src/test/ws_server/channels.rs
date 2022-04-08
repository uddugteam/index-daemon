use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::error_type::ErrorType;
use crate::test::ws_server::helper_functions::{
    check_incoming_messages, check_subscriptions, extract_subscription_request, make_unsub_request,
    start_application, ws_connect_and_send, SubscriptionsExpected,
};
use crate::test::ws_server::request::{Request, Requests, RequestsUnzipped};
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use serial_test::serial;
use std::collections::HashMap;
use std::thread;
use std::time;
use uuid::Uuid;

#[test]
#[ignore]
#[serial]
fn test_add_ws_channels_separately() {
    let mut port = 8100;
    let mut config = ConfigScheme::default();
    let channels = WsChannelName::get_all_channels();

    for request in Requests::make_all(&config, &channels, false, None).0 {
        let Request {
            request,
            params_item,
            expected,
        } = request;

        config.service.ws_addr = format!("127.0.0.1:{}", port);
        port += 1;

        let expected = expected
            .map(extract_subscription_request)
            .map(Option::unwrap);

        let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
            start_application(config.clone());

        ws_connect_and_send(&config.service.ws_addr, vec![request], incoming_msg_tx);
        let res = check_subscriptions(&repositories_prepared, vec![(params_item, expected)]);
        if res.is_err() {
            panic!("Expected Ok. Got: {:#?}", res);
        }
    }
}

#[test]
#[ignore]
#[serial]
fn test_add_ws_channels_separately_with_errors() {
    let mut port = 8200;
    let mut config = ConfigScheme::default();
    let channels = WsChannelName::get_all_channels();

    for request in Requests::make_all(&config, &channels, true, None).0 {
        let Request {
            request,
            params_item,
            expected,
        } = request;

        config.service.ws_addr = format!("127.0.0.1:{}", port);
        port += 1;

        let expected = expected
            .map(extract_subscription_request)
            .map(Option::unwrap);

        let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
            start_application(config.clone());

        ws_connect_and_send(
            &config.service.ws_addr,
            vec![request.clone()],
            incoming_msg_tx,
        );
        let res = check_subscriptions(
            &repositories_prepared,
            vec![(params_item.clone(), expected.clone())],
        );
        if res.is_ok() {
            panic!("Expected Err. Got: {:#?}", res);
        }
    }
}

#[test]
#[ignore]
#[serial]
fn test_add_ws_channels_together() {
    let port = 8300;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let RequestsUnzipped {
        requests,
        params_vec,
        expecteds,
    } = Requests::make_all(&config, &channels, false, None).unzip();

    let expecteds: Vec<Result<WsChannelSubscriptionRequest, ErrorType>> = expecteds
        .into_iter()
        .map(|v| v.map(extract_subscription_request).map(Option::unwrap))
        .collect();

    let expecteds = params_vec.into_iter().zip(expecteds).collect();

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);
    let res = check_subscriptions(&repositories_prepared, expecteds);
    if res.is_err() {
        panic!("Expected Ok. Got: {:#?}", res);
    }
}

#[test]
#[ignore]
#[serial]
fn test_add_ws_channels_together_with_errors() {
    let port = 8400;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let RequestsUnzipped {
        requests,
        params_vec,
        expecteds,
    } = Requests::make_all(&config, &channels, true, None).unzip();

    let expecteds: Vec<Result<WsChannelSubscriptionRequest, ErrorType>> = expecteds
        .into_iter()
        .map(|v| v.map(extract_subscription_request).map(Option::unwrap))
        .collect();

    let expecteds = params_vec.into_iter().zip(expecteds).collect();

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);
    let res = check_subscriptions(&repositories_prepared, expecteds);
    if res.is_ok() {
        panic!("Expected Err. Got: {:#?}", res);
    }
}

#[test]
#[ignore]
#[serial]
fn test_resubscribe() {
    let port = 8500;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();
    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());

    let RequestsUnzipped {
        requests,
        params_vec,
        expecteds,
    } = Requests::make_all(&config, &channels, false, Some(sub_id)).unzip();

    let expecteds: Vec<Result<WsChannelSubscriptionRequest, ErrorType>> = expecteds
        .into_iter()
        .map(|v| v.map(extract_subscription_request).map(Option::unwrap))
        .collect();

    let mut expecteds: Vec<SubscriptionsExpected> = params_vec.into_iter().zip(expecteds).collect();

    // Remove last
    let last_expected = expecteds.swap_remove(expecteds.len() - 1);

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);

    // Wait until all requests are processed
    thread::sleep(time::Duration::from_millis(5000));

    // Check that all except last are NOT subscribed
    let res = check_subscriptions(&repositories_prepared, expecteds);
    if res.is_ok() {
        panic!("Expected Err. Got: {:#?}", res);
    }

    // Check that last IS subscribed
    let res = check_subscriptions(&repositories_prepared, vec![last_expected]);
    if res.is_err() {
        panic!("Expected Ok. Got: {:#?}", res);
    }
}

#[test]
#[ignore]
#[serial]
fn test_unsubscribe() {
    let port = 8600;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let subscription_requests = Requests::make_all(&config, &channels, false, None);

    for request in subscription_requests.0 {
        let Request {
            request,
            params_item,
            expected,
        } = request;

        let sub_id = params_item.id.clone();
        let unsub_request = make_unsub_request(sub_id);
        let requests = vec![request, unsub_request];

        let expected = expected
            .map(extract_subscription_request)
            .map(Option::unwrap);

        ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx.clone());
        let res = check_subscriptions(
            &repositories_prepared,
            vec![(params_item.clone(), expected.clone())],
        );
        if res.is_ok() {
            panic!("Expected Err. Got: {:#?}", res);
        }
    }
}

#[test]
#[ignore]
#[serial]
fn test_channels_response_together() {
    let port = 8700;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let RequestsUnzipped {
        requests,
        params_vec: _,
        expecteds,
    } = Requests::make_all(&config, &channels, false, None).unzip();

    let expecteds = expecteds
        .into_iter()
        .map(|v| extract_subscription_request(v.unwrap()).unwrap())
        .map(|v| (v.get_id(), v.get_method()))
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
fn test_channels_response_together_with_errors() {
    let port = 8700;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let RequestsUnzipped {
        requests,
        params_vec,
        expecteds,
    } = Requests::make_all(&config, &channels, true, None).unzip();

    let expecteds_errors: HashMap<_, _> = params_vec
        .iter()
        .zip(expecteds)
        .map(|v| (v.0.id.clone(), v.1.unwrap_err()))
        .collect();

    let expecteds_new = params_vec.into_iter().map(|v| (v.id, v.channel)).collect();

    ws_connect_and_send(&config.service.ws_addr, requests, incoming_msg_tx);
    let (hash_map, no_id) = check_incoming_messages(incoming_msg_rx, &expecteds_new);

    let mut expecteds_with_no_ids = Vec::new();
    for (id, method) in expecteds_new {
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
