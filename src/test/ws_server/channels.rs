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
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_add_ws_channels_together() {
    let port = 8300;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (worker_future, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
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

    let ws_client_future = ws_connect_and_send(
        config.service.ws_addr.to_string(),
        requests,
        incoming_msg_tx,
    );
    let check_subscriptions_future = check_subscriptions(repositories_prepared, expecteds);

    // To prevent DDoS attack on exchanges
    sleep(Duration::from_millis(3000)).await;

    tokio::select! {
        _ = worker_future => {}
        _ = ws_client_future => {}
        res = check_subscriptions_future => {
            if res.is_err() {
                panic!("Expected Ok. Got: {:#?}", res);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_add_ws_channels_together_with_errors() {
    let port = 8400;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (worker_future, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
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

    let ws_client_future = ws_connect_and_send(
        config.service.ws_addr.to_string(),
        requests,
        incoming_msg_tx,
    );
    let check_subscriptions_future = check_subscriptions(repositories_prepared, expecteds);

    // To prevent DDoS attack on exchanges
    sleep(Duration::from_millis(3000)).await;

    tokio::select! {
        _ = worker_future => {}
        _ = ws_client_future => {}
        res = check_subscriptions_future => {
            if res.is_ok() {
                panic!("Expected Err. Got: {:#?}", res);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_resubscribe() {
    let port = 8500;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (worker_future, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
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

    let ws_client_future = ws_connect_and_send(
        config.service.ws_addr.to_string(),
        requests,
        incoming_msg_tx,
    );

    let repositories_prepared_2 = repositories_prepared.clone();

    // Check that all except last are NOT subscribed
    let check_subscriptions_future = async move {
        // Wait until all requests are processed
        sleep(Duration::from_millis(5000)).await;

        // Check that all except last are NOT subscribed
        check_subscriptions(repositories_prepared_2, expecteds).await
    };

    // To prevent DDoS attack on exchanges
    sleep(Duration::from_millis(3000)).await;

    tokio::select! {
        _ = worker_future => {}
        _ = ws_client_future => {}
        res = check_subscriptions_future => {
            // Check that all except last are NOT subscribed
            if res.is_ok() {
                panic!("Expected Err. Got: {:#?}", res);
            }

            // Check that last IS subscribed
            let res = check_subscriptions(repositories_prepared, vec![last_expected]).await;
            if res.is_err() {
                panic!("Expected Ok. Got: {:#?}", res);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_unsubscribe() {
    let port = 8600;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let mut futures = Vec::new();

    let (worker_future, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let subscription_requests = Requests::make_all(&config, &channels, false, None);

    let mut sleep_seconds = 3;
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

        let ws_client_future = ws_connect_and_send(
            config.service.ws_addr.to_string(),
            requests,
            incoming_msg_tx.clone(),
        );

        let check_subscriptions_future = check_subscriptions(
            repositories_prepared.clone(),
            vec![(params_item.clone(), expected.clone())],
        );

        let future = async move {
            tokio::select! {
                _ = ws_client_future => {}
                res = check_subscriptions_future => {
                    if res.is_ok() {
                        panic!("Expected Err. Got: {:#?}", res);
                    }
                }
            }
        };

        futures.push(future);

        sleep_seconds += 3;
    }

    // To prevent DDoS attack on exchanges
    sleep(Duration::from_secs(sleep_seconds)).await;

    tokio::select! {
        _ = worker_future => {}
        _ = futures::future::join_all(futures) => {}
    }
}

#[tokio::test]
#[serial]
async fn test_channels_response_together() {
    let port = 8700;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (worker_future, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
        start_application(config);

    let channels = WsChannelName::get_all_channels();

    let RequestsUnzipped {
        requests,
        params_vec: _,
        expecteds,
    } = Requests::make_all(&config, &channels, false, None).unzip();

    let expecteds: HashMap<_, _> = expecteds
        .into_iter()
        .map(|v| extract_subscription_request(v.unwrap()).unwrap())
        .map(|v| (v.get_id(), v.get_method()))
        .collect();

    let ws_client_future = ws_connect_and_send(
        config.service.ws_addr.to_string(),
        requests,
        incoming_msg_tx,
    );
    let check_incoming_messages_future =
        check_incoming_messages(incoming_msg_rx, expecteds.clone());

    // To prevent DDoS attack on exchanges
    sleep(Duration::from_millis(3000)).await;

    tokio::select! {
        _ = worker_future => {}
        _ = ws_client_future => {}
        (hash_map, no_id) = check_incoming_messages_future => {
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
    }
}

#[tokio::test]
#[serial]
async fn test_channels_response_together_with_errors() {
    let port = 8800;
    let mut config = ConfigScheme::default();
    config.service.ws_addr = format!("127.0.0.1:{}", port);

    let (worker_future, (incoming_msg_tx, incoming_msg_rx), config, _repositories_prepared) =
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

    let expecteds: HashMap<_, _> = params_vec.into_iter().map(|v| (v.id, v.channel)).collect();

    let ws_client_future = ws_connect_and_send(
        config.service.ws_addr.to_string(),
        requests,
        incoming_msg_tx,
    );
    let check_incoming_messages_future =
        check_incoming_messages(incoming_msg_rx, expecteds.clone());

    // To prevent DDoS attack on exchanges
    sleep(Duration::from_millis(3000)).await;

    tokio::select! {
        _ = worker_future => {}
        _ = ws_client_future => {}
        (hash_map, no_id) = check_incoming_messages_future => {
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
    }
}
