use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::error_type::ErrorType;
use crate::test::ws_server::helper_functions::{
    check_subscriptions, extract_subscription_request, make_unsub_request, start_application,
    ws_connect_and_send, SubscriptionsExpected,
};
use crate::test::ws_server::request::{Request, Requests, RequestsUnzipped};
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use serial_test::serial;
use std::thread;
use std::time;
use uuid::Uuid;

#[test]
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
            panic!("Expected Ok. Got: {:?}", res);
        }
    }
}

#[test]
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
            panic!("Expected Err. Got: {:?}", res);
        }
    }
}

#[test]
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
        panic!("Expected Ok. Got: {:?}", res);
    }
}

#[test]
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
        panic!("Expected Err. Got: {:?}", res);
    }
}

#[test]
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
        panic!("Expected Err. Got: {:?}", res);
    }

    // Check that last IS subscribed
    let res = check_subscriptions(&repositories_prepared, vec![last_expected]);
    if res.is_err() {
        panic!("Expected Ok. Got: {:?}", res);
    }
}

#[test]
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
            panic!("Expected Err. Got: {:?}", res);
        }
    }
}