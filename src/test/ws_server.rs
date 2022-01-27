mod ws_client_for_testing;

use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::ws_client_for_testing::WsClientForTesting;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::worker::test::check_worker_subscriptions;
use crate::worker::worker::Worker;
use serde_json::json;
use serial_test::serial;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time;
use uuid::Uuid;

fn start_application(ws_addr: &str) -> (Receiver<JoinHandle<()>>, Arc<Mutex<Worker>>) {
    // To prevent DDoS attack on exchanges
    thread::sleep(time::Duration::from_millis(3000));

    let graceful_shutdown = Arc::new(Mutex::new(false));

    let mut config = ConfigScheme::default();
    config.service.ws = true;
    config.service.ws_addr = ws_addr.to_string();
    config.market.channels = vec![MarketChannels::Trades];

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx, graceful_shutdown);
    worker.lock().unwrap().start(config);

    // Give Websocket server time to start
    thread::sleep(time::Duration::from_millis(1000));

    (rx, worker)
}

fn make_request(
    sub_id: &str,
    method: &str,
    coins: &[String],
    exchanges: Option<Vec<String>>,
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

fn make_unsub_request(method: &str) -> String {
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

fn ws_connect_and_send_messages(ws_addr: &str, messages: Vec<String>) {
    let uri = "ws://".to_string() + ws_addr;

    // Do not join
    let _ = thread::spawn(|| {
        let ws_client = WsClientForTesting::new(uri, messages, |_info: String| {});
        ws_client.start();
    });

    // Give Websocket server time to process request
    thread::sleep(time::Duration::from_millis(1000));
}

#[test]
#[serial]
fn test_worker_add_ws_channel() {
    let ws_addr = "127.0.0.1:8001";
    let (_rx, worker) = start_application(ws_addr);

    let method = "coin_average_price".to_string();
    let sub_id = Uuid::new_v4().to_string();
    let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();

    let request = make_request(&sub_id, &method, &coins, None);

    ws_connect_and_send_messages(ws_addr, vec![request]);

    check_worker_subscriptions(&worker, vec![(sub_id, method, coins)]);
}

#[test]
#[serial]
fn test_worker_resub_ws_channel() {
    // Resubscribe

    let ws_addr = "127.0.0.1:8002";
    let (_rx, worker) = start_application(ws_addr);

    let mut requests = Vec::new();
    let method = "coin_average_price".to_string();

    let sub_id = Uuid::new_v4().to_string();
    let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();
    let request = make_request(&sub_id, &method, &coins, None);
    requests.push(request);

    let sub_id = Uuid::new_v4().to_string();
    let coins = ["BTC".to_string()].to_vec();
    let request = make_request(&sub_id, &method, &coins, None);
    requests.push(request);

    ws_connect_and_send_messages(ws_addr, requests);

    check_worker_subscriptions(&worker, vec![(sub_id, method, coins)]);
}

#[test]
#[serial]
fn test_worker_unsub_ws_channel() {
    // Unsubscribe

    let ws_addr = "127.0.0.1:8003";
    let (_rx, worker) = start_application(ws_addr);

    let mut requests = Vec::new();

    let method = "coin_average_price".to_string();
    let sub_id = Uuid::new_v4().to_string();
    let coins = ["BTC".to_string(), "ETH".to_string()].to_vec();
    let request = make_request(&sub_id, &method, &coins, None);
    requests.push(request);

    let request = make_unsub_request(&method);
    requests.push(request);

    ws_connect_and_send_messages(ws_addr, requests);

    check_worker_subscriptions(&worker, Vec::new());
}
