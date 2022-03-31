mod ws_client_for_testing;

use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
use crate::test::ws_server::ws_client_for_testing::WsClientForTesting;
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::network_helpers::ws_server::channels::market_channels::LocalMarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::LocalWorkerChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::worker::start_worker;
use parse_duration::parse;
use serde_json::json;
use serial_test::serial;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time;
use uuid::Uuid;

fn start_application(
    ws_addr: &str,
) -> (
    Receiver<JoinHandle<()>>,
    (Sender<String>, Receiver<String>),
    RepositoriesPrepared,
) {
    // To prevent DDoS attack on exchanges
    thread::sleep(time::Duration::from_millis(3000));

    let graceful_shutdown = Arc::new(RwLock::new(false));

    let mut config = ConfigScheme::default();
    config.service.ws = true;
    config.service.ws_addr = ws_addr.to_string();
    config.market.channels = vec![ExternalMarketChannels::Trades];

    let (tx, rx) = mpsc::channel();
    let repositories_prepared = start_worker(config, tx, graceful_shutdown);

    // Give Websocket server time to start
    thread::sleep(time::Duration::from_millis(1000));

    (rx, mpsc::channel(), repositories_prepared)
}

fn make_request(
    sub_id: &JsonRpcId,
    method: WsChannelName,
    coins: Option<&[String]>,
    exchanges: Option<&[String]>,
    interval: Option<String>,
) -> String {
    let request = json!({
        "id": sub_id,
        "jsonrpc": "2.0",
        "method": method,
        "params": {
          "coins": coins,
          "exchanges": exchanges,
          "interval": interval
        }
    });

    serde_json::to_string(&request).unwrap()
}

fn make_unsub_request(sub_id: &str) -> String {
    let request = json!({
        "id": sub_id,
        "jsonrpc": "2.0",
        "method": "unsubscribe",
        "params": {}
    });

    serde_json::to_string(&request).unwrap()
}

fn ws_connect_and_subscribe(
    ws_addr: &str,
    outgoing_messages: Vec<String>,
    incoming_msg_tx: Sender<String>,
) {
    let uri = "ws://".to_string() + ws_addr;

    // Do not join
    let _ = thread::spawn(move || {
        let ws_client = WsClientForTesting::new(uri, outgoing_messages, |msg: String| {
            let _ = incoming_msg_tx.send(msg);
        });
        ws_client.start();
    });

    // Give Websocket server time to process request
    thread::sleep(time::Duration::from_millis(1000));
}

fn zip_coins_with_usd(coins: Option<&[String]>) -> Vec<Option<(String, String)>> {
    coins
        .map(|v| {
            v.iter()
                .map(|coin| (coin.to_string(), "USD".to_string()))
                .map(Some)
                .collect()
        })
        .unwrap_or(vec![None])
}

fn get_subscription_requests(
    repositories_prepared: &RepositoriesPrepared,
    sub_id: &JsonRpcId,
    method: WsChannelName,
    pairs: &[Option<(String, String)>],
    exchanges: &[String],
) -> Vec<WsChannelSubscriptionRequest> {
    let mut subscription_requests = Vec::new();

    for pair in pairs {
        for exchange in exchanges {
            let key = (
                exchange.to_string(),
                method.get_market_value(),
                pair.clone(),
            );
            let ws_channels = repositories_prepared
                .ws_channels_holder
                .get(&key)
                .unwrap()
                .read()
                .unwrap();

            ws_channels
                .get_channels_by_method(method)
                .into_iter()
                .filter(|(k, _)| &k.1 == sub_id)
                .map(|(_, v)| v)
                .for_each(|v| subscription_requests.push(v.clone()));
        }
    }

    subscription_requests
}

fn prepare_some_params(
    coins: Option<&[String]>,
    exchanges: Option<&[String]>,
) -> (Vec<Option<(String, String)>>, Vec<String>) {
    let pairs = zip_coins_with_usd(coins);
    let exchanges = exchanges
        .map(|v| v.to_vec())
        .unwrap_or(vec!["worker".to_string()]);

    (pairs, exchanges)
}

fn check_subscriptions(
    repositories_prepared: RepositoriesPrepared,
    subscriptions: Vec<WsChannelSubscriptionRequest>,
) -> Result<(), String> {
    for subscription_request_expected in subscriptions {
        let sub_id_expected = subscription_request_expected.get_id();
        let method_expected = subscription_request_expected.get_method();
        let coins_expected = subscription_request_expected.get_coins();
        let exchanges_expected = subscription_request_expected.get_exchanges();

        let (pairs_expected, exchanges_expected) =
            prepare_some_params(coins_expected, exchanges_expected);

        let expected_len = pairs_expected.len() * exchanges_expected.len();

        let subscription_requests = get_subscription_requests(
            &repositories_prepared,
            &sub_id_expected,
            method_expected,
            &pairs_expected,
            &exchanges_expected,
        );
        if subscription_requests.len() != expected_len {
            return Err(format!(
                "Error: subscription_requests.len() != expected_len. Got: {}, Expected: {}. Method: {:?}",
                subscription_requests.len(), expected_len, method_expected,
            ));
        }

        for subscription_request in subscription_requests {
            if subscription_request != subscription_request_expected {
                return Err(format!(
                    "Error: subscription_request != subscription_request_expected. Got: {:?}, Expected: {:?}. Method: {:?}",
                    subscription_request, subscription_request_expected, method_expected,
                ));
            }
        }
    }

    Ok(())
}

fn get_all_subscription_requests() -> Vec<(String, WsChannelSubscriptionRequest)> {
    let mut subscription_requests = Vec::new();

    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let method = WsChannelName::IndexPrice;
    let percent_change_interval = "1minute".to_string();
    let request = make_request(&sub_id, method, None, None, None);
    let expected = WsChannelSubscriptionRequest::Worker(LocalWorkerChannels::IndexPrice {
        id: sub_id,
        frequency_ms: 100,
        percent_change_interval_sec: parse(&percent_change_interval).unwrap().as_secs(),
    });
    subscription_requests.push((request, expected));

    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let method = WsChannelName::IndexPriceCandles;
    let interval = "1day".to_string();
    let request = make_request(&sub_id, method, None, None, Some(interval.clone()));
    let expected = WsChannelSubscriptionRequest::Worker(LocalWorkerChannels::IndexPriceCandles {
        id: sub_id,
        frequency_ms: 100,
        interval_sec: parse(&interval).unwrap().as_secs(),
    });
    subscription_requests.push((request, expected));

    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let method = WsChannelName::CoinAveragePrice;
    let coins = vec!["BTC".to_string(), "ETH".to_string()];
    let percent_change_interval = "1minute".to_string();
    let request = make_request(&sub_id, method, Some(&coins), None, None);
    let expected = WsChannelSubscriptionRequest::Worker(LocalWorkerChannels::CoinAveragePrice {
        id: sub_id,
        coins,
        frequency_ms: 100,
        percent_change_interval_sec: parse(&percent_change_interval).unwrap().as_secs(),
    });
    subscription_requests.push((request, expected));

    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let method = WsChannelName::CoinAveragePriceCandles;
    let coins = vec!["BTC".to_string(), "ETH".to_string()];
    let interval = "1day".to_string();
    let request = make_request(&sub_id, method, Some(&coins), None, Some(interval.clone()));
    let expected =
        WsChannelSubscriptionRequest::Worker(LocalWorkerChannels::CoinAveragePriceCandles {
            id: sub_id,
            coins,
            frequency_ms: 100,
            interval_sec: parse(&interval).unwrap().as_secs(),
        });
    subscription_requests.push((request, expected));

    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let method = WsChannelName::CoinExchangePrice;
    let coins = vec!["BTC".to_string(), "ETH".to_string()];
    let exchanges = vec!["binance".to_string(), "coinbase".to_string()];
    let percent_change_interval = "1minute".to_string();
    let request = make_request(&sub_id, method, Some(&coins), Some(&exchanges), None);
    let expected = WsChannelSubscriptionRequest::Market(LocalMarketChannels::CoinExchangePrice {
        id: sub_id,
        coins,
        exchanges,
        frequency_ms: 100,
        percent_change_interval_sec: parse(&percent_change_interval).unwrap().as_secs(),
    });
    subscription_requests.push((request, expected));

    let sub_id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let method = WsChannelName::CoinExchangeVolume;
    let coins = vec!["BTC".to_string(), "ETH".to_string()];
    let exchanges = vec!["binance".to_string(), "coinbase".to_string()];
    let percent_change_interval = "1minute".to_string();
    let request = make_request(&sub_id, method, Some(&coins), Some(&exchanges), None);
    let expected = WsChannelSubscriptionRequest::Market(LocalMarketChannels::CoinExchangeVolume {
        id: sub_id,
        coins,
        exchanges,
        frequency_ms: 100,
        percent_change_interval_sec: parse(&percent_change_interval).unwrap().as_secs(),
    });
    subscription_requests.push((request, expected));

    subscription_requests
}

fn get_all_subscription_requests_unzipped() -> (Vec<String>, Vec<WsChannelSubscriptionRequest>) {
    let mut requests = Vec::new();
    let mut expecteds = Vec::new();

    for (request, expected) in get_all_subscription_requests() {
        requests.push(request);
        expecteds.push(expected);
    }

    (requests, expecteds)
}

#[test]
#[serial]
fn test_add_ws_channels_separately() {
    let mut port = 8100;
    for (request, expected) in get_all_subscription_requests() {
        let ws_addr = format!("127.0.0.1:{}", port);
        port += 1;

        let (_rx, (incoming_msg_tx, _incoming_msg_rx), repositories_prepared) =
            start_application(&ws_addr);

        ws_connect_and_subscribe(&ws_addr, vec![request], incoming_msg_tx);
        check_subscriptions(repositories_prepared, vec![expected]).unwrap();
    }
}

#[test]
#[serial]
fn test_add_ws_channels_together() {
    let port = 8200;
    let ws_addr = format!("127.0.0.1:{}", port);

    let (_rx, (incoming_msg_tx, _incoming_msg_rx), repositories_prepared) =
        start_application(&ws_addr);

    let (requests, expecteds) = get_all_subscription_requests_unzipped();

    ws_connect_and_subscribe(&ws_addr, requests, incoming_msg_tx);
    check_subscriptions(repositories_prepared, expecteds).unwrap();
}
