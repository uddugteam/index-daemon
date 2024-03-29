mod ws_client_for_testing;

use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::ws_client_for_testing::WsClientForTesting;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::worker::Worker;
use serde_json::json;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time;
use std::time::Instant;
use uuid::Uuid;

fn start_application(
    ws_addr: &str,
) -> (
    Receiver<JoinHandle<()>>,
    Worker,
    (Sender<String>, Receiver<String>),
) {
    // To prevent DDoS attack on exchanges
    thread::sleep(time::Duration::from_millis(3000));

    let graceful_shutdown = Arc::new(Mutex::new(false));

    let mut config = ConfigScheme::default();
    config.service.ws = true;
    config.service.ws_addr = ws_addr.to_string();
    config.market.channels = vec![MarketChannels::Trades];

    let (tx, rx) = mpsc::channel();
    let mut worker = Worker::new(tx, graceful_shutdown);
    worker.start(config);

    // Give Websocket server time to start
    thread::sleep(time::Duration::from_millis(1000));

    (rx, worker, mpsc::channel())
}

fn make_request(
    sub_id: &str,
    method: WsChannelName,
    coins: &[String],
    exchanges: Option<&Vec<String>>,
) -> String {
    let request = match exchanges {
        Some(exchanges) => json!({
            "id": sub_id,
            "jsonrpc": "2.0",
            "method": method,
            "params": {
              "coins": coins,
              "exchanges": exchanges,
              "frequency_ms": 100u64
            }
        }),
        None => json!({
            "id": sub_id,
            "jsonrpc": "2.0",
            "method": method,
            "params": {
              "coins": coins,
              "frequency_ms": 100u64
            }
        }),
    };

    serde_json::to_string(&request).unwrap()
}

fn make_unsub_request(method: WsChannelName) -> String {
    let request = json!({
        "id": null,
        "jsonrpc": "2.0",
        "method": "unsubscribe",
        "params": {
          "method": method
        }
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

fn check_incoming_messages(
    incoming_msg_rx: Receiver<String>,
    expected: Vec<(String, WsChannelName, Vec<String>, Option<Vec<String>>)>,
) {
    let mut methods = HashMap::new();
    for (sub_id, method, ..) in &expected {
        methods.insert(sub_id.to_string(), *method);
    }

    let mut expected_new: HashMap<(String, WsChannelName, String, Option<String>), ()> =
        HashMap::new();
    for (sub_id, method, coins, exchanges) in expected {
        for coin in coins {
            if let Some(exchanges) = exchanges.clone() {
                for exchange in exchanges {
                    expected_new.insert((sub_id.clone(), method, coin.clone(), Some(exchange)), ());
                }
            } else {
                expected_new.insert((sub_id.clone(), method, coin, None), ());
            }
        }
    }

    let start = Instant::now();
    while !expected_new.is_empty() {
        if let Ok(incoming_msg) = incoming_msg_rx.try_recv() {
            let incoming_msg: serde_json::Value = serde_json::from_str(&incoming_msg).unwrap();

            let sub_id = incoming_msg.get("id").unwrap().as_str().unwrap();
            let jsonrpc = incoming_msg.get("jsonrpc").unwrap().as_str().unwrap();
            assert_eq!(jsonrpc, "2.0");

            let result = incoming_msg.get("result").unwrap().as_object().unwrap();
            if let Some(message) = result.get("message") {
                // SuccSub message (`result` has `message` field)

                let message = message.as_str().unwrap();
                assert_eq!(message, "Successfully subscribed.");
            } else {
                // Message with payload (no `message` field in `result`)

                let coin = result.get("coin").unwrap().as_str().unwrap().to_string();
                let exchange = result
                    .get("exchange")
                    .map(|v| v.as_str().unwrap().to_string());
                let _value = result.get("value").unwrap().as_f64().unwrap();
                let timestamp = result.get("timestamp").unwrap().as_i64().unwrap();
                // 1640984400 = 2022-01-01 00:00:00
                assert!(timestamp > 1640984400);
                let method = *methods.get(sub_id).unwrap();

                expected_new.remove(&(sub_id.to_string(), method, coin, exchange));
            }
        }

        let minutes = 4;
        if start.elapsed().as_secs() > minutes * 60 {
            panic!("Allocated time ({} minutes) is over.", minutes);
        }

        thread::sleep(time::Duration::from_millis(100));
    }
}

#[test]
#[ignore]
#[serial]
/// TODO: Fix (not always working on github)
fn test_ws_channels_response() {
    let ws_addr = "127.0.0.1:8000";
    let (_rx, _worker, (incoming_msg_tx, incoming_msg_rx)) = start_application(ws_addr);

    let mut requests = Vec::new();
    let mut expected = Vec::new();

    let sub_id = Uuid::new_v4().to_string();
    let method = WsChannelName::CoinAveragePrice;
    let coins = ["BTC".to_string()].to_vec();
    let request = make_request(&sub_id, method, &coins, None);
    requests.push(request);
    expected.push((sub_id, method, coins, None));

    let sub_id = Uuid::new_v4().to_string();
    let method = WsChannelName::CoinExchangePrice;
    let coins = ["BTC".to_string()].to_vec();
    let exchanges = ["binance".to_string()].to_vec();
    let request = make_request(&sub_id, method, &coins, Some(&exchanges));
    requests.push(request);
    expected.push((sub_id, method, coins, Some(exchanges)));

    let sub_id = Uuid::new_v4().to_string();
    let method = WsChannelName::CoinExchangeVolume;
    let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();
    let exchanges = ["binance".to_string(), "coinbase".to_string()].to_vec();
    let request = make_request(&sub_id, method, &coins, Some(&exchanges));
    requests.push(request);
    expected.push((sub_id, method, coins, Some(exchanges)));

    ws_connect_and_subscribe(ws_addr, requests, incoming_msg_tx);
    check_incoming_messages(incoming_msg_rx, expected);
}
