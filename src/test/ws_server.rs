mod ws_client_for_testing;

use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
use crate::test::error_type::{ErrorType, Field};
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

#[derive(Debug, Clone)]
struct SubscriptionParams {
    pub id: JsonRpcId,
    pub channel: WsChannelName,
    pub coins: Option<Vec<String>>,
    pub exchanges: Option<Vec<String>>,
}

fn start_application(
    mut config: ConfigScheme,
) -> (
    Receiver<JoinHandle<()>>,
    (Sender<String>, Receiver<String>),
    ConfigScheme,
    RepositoriesPrepared,
) {
    // To prevent DDoS attack on exchanges
    thread::sleep(time::Duration::from_millis(3000));

    let graceful_shutdown = Arc::new(RwLock::new(false));

    config.service.ws = true;
    config.market.channels = vec![ExternalMarketChannels::Trades];

    let (tx, rx) = mpsc::channel();
    let repositories_prepared = start_worker(config.clone(), tx, graceful_shutdown);

    // Give Websocket server time to start
    thread::sleep(time::Duration::from_millis(1000));

    (rx, mpsc::channel(), config, repositories_prepared)
}

fn spoil_request(config: &ConfigScheme, request: &mut serde_json::Value, error: Option<ErrorType>) {
    // There we damage the request according to ErrorType
    if let Some(error) = error {
        let request_object = request.as_object_mut().unwrap();

        let params_object = request_object.get_mut("params").unwrap();
        let params_object = params_object.as_object_mut().unwrap();

        let object = match error {
            ErrorType::Lack(field)
            | ErrorType::Null(field)
            | ErrorType::Empty(field)
            | ErrorType::InvalidType(field)
            | ErrorType::InvalidValue(field)
            | ErrorType::Low(field)
            | ErrorType::Unavailable(field) => {
                if field.is_root() {
                    request_object
                } else {
                    params_object
                }
            }
        };

        match error {
            ErrorType::Lack(field) => {
                object.remove(&field.to_string());
            }
            ErrorType::Null(field) => {
                object.insert(field.to_string(), serde_json::Value::Null);
            }
            ErrorType::Empty(field) => {
                let value = object.get(&field.to_string()).unwrap();

                match value {
                    serde_json::Value::Array(..) => {
                        object.insert(
                            field.to_string(),
                            serde_json::Value::Object(serde_json::Map::new()),
                        );
                    }
                    serde_json::Value::Object(..) => {
                        object.insert(field.to_string(), serde_json::Value::Array(Vec::new()));
                    }
                    _ => unreachable!(),
                }
            }
            ErrorType::InvalidType(field) => {
                let value = object.get(&field.to_string()).unwrap();

                match value {
                    serde_json::Value::Array(..) => {
                        object.insert(
                            field.to_string(),
                            serde_json::Value::Object(serde_json::Map::new()),
                        );
                    }
                    serde_json::Value::Object(..) => {
                        object.insert(field.to_string(), serde_json::Value::Array(Vec::new()));
                    }
                    serde_json::Value::Number(..) => {
                        object.insert(
                            field.to_string(),
                            serde_json::Value::String("some_string".to_string()),
                        );
                    }
                    serde_json::Value::String(..) => {
                        object.insert(field.to_string(), serde_json::Value::Number(123.into()));
                    }
                    _ => unreachable!(),
                }
            }
            ErrorType::InvalidValue(field) => {
                let value = object.get(&field.to_string()).unwrap();

                match value {
                    serde_json::Value::String(..) => {
                        object.insert(
                            field.to_string(),
                            serde_json::Value::String("invalid_duration_format".to_string()),
                        );
                    }
                    _ => unreachable!(),
                }
            }
            ErrorType::Low(field) => match field {
                Field::FrequencyMs => {
                    let low_frequency_ms = config.service.ws_answer_timeout_ms - 1;

                    object.insert(
                        field.to_string(),
                        serde_json::Value::Number(low_frequency_ms.into()),
                    );
                }
                Field::PercentChangeInterval | Field::Interval => {
                    object.insert(
                        field.to_string(),
                        serde_json::Value::String("999 millisecond".to_string()),
                    );
                }
                _ => unreachable!(),
            },
            ErrorType::Unavailable(field) => {
                let value = object.get(&field.to_string()).unwrap();

                match value {
                    serde_json::Value::Array(..) => {
                        object.insert(
                            field.to_string(),
                            serde_json::Value::Array(vec![serde_json::Value::String(
                                "unavailable_coin_or_exchange".to_string(),
                            )]),
                        );
                    }
                    serde_json::Value::String(..) => {
                        object.insert(
                            field.to_string(),
                            serde_json::Value::String("unavailable_method".to_string()),
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

fn make_request(
    config: &ConfigScheme,
    method: WsChannelName,
    error: Option<ErrorType>,
) -> (
    String,
    SubscriptionParams,
    Result<WsChannelSubscriptionRequest, ErrorType>,
) {
    let id = JsonRpcId::Str(Uuid::new_v4().to_string());
    let coins = vec!["BTC".to_string(), "ETH".to_string()];
    let exchanges = vec!["binance".to_string(), "coinbase".to_string()];
    let frequency_ms = 100;
    let percent_change_interval = "1minute".to_string();
    let interval = "1day".to_string();

    let mut request = json!({
        "id": id,
        "jsonrpc": "2.0",
        "method": method,
        "params": {
          "coins": coins,
          "exchanges": exchanges,
          "frequency_ms": frequency_ms,
          "percent_change_interval": percent_change_interval,
          "interval": interval
        }
    });

    spoil_request(config, &mut request, error);

    let request = serde_json::to_string(&request).unwrap();

    let expected = if let Some(error) = error {
        Err(error)
    } else {
        let id = id.clone();
        let coins = coins.clone();
        let exchanges = exchanges.clone();
        let percent_change_interval_sec = parse(&percent_change_interval).unwrap().as_secs();
        let interval_sec = parse(&interval).unwrap().as_secs();

        match method {
            WsChannelName::IndexPrice => Ok(WsChannelSubscriptionRequest::Worker(
                LocalWorkerChannels::IndexPrice {
                    id,
                    frequency_ms,
                    percent_change_interval_sec,
                },
            )),
            WsChannelName::IndexPriceCandles => Ok(WsChannelSubscriptionRequest::Worker(
                LocalWorkerChannels::IndexPriceCandles {
                    id,
                    frequency_ms,
                    interval_sec,
                },
            )),
            WsChannelName::CoinAveragePrice => Ok(WsChannelSubscriptionRequest::Worker(
                LocalWorkerChannels::CoinAveragePrice {
                    id,
                    coins,
                    frequency_ms,
                    percent_change_interval_sec,
                },
            )),
            WsChannelName::CoinAveragePriceCandles => Ok(WsChannelSubscriptionRequest::Worker(
                LocalWorkerChannels::CoinAveragePriceCandles {
                    id,
                    coins,
                    frequency_ms,
                    interval_sec,
                },
            )),
            WsChannelName::CoinExchangePrice => Ok(WsChannelSubscriptionRequest::Market(
                LocalMarketChannels::CoinExchangePrice {
                    id,
                    coins,
                    exchanges,
                    frequency_ms,
                    percent_change_interval_sec,
                },
            )),
            WsChannelName::CoinExchangeVolume => Ok(WsChannelSubscriptionRequest::Market(
                LocalMarketChannels::CoinExchangeVolume {
                    id,
                    coins,
                    exchanges,
                    frequency_ms,
                    percent_change_interval_sec,
                },
            )),
            _ => unreachable!(),
        }
    };

    let coins_expected = match method {
        WsChannelName::CoinAveragePrice
        | WsChannelName::CoinAveragePriceCandles
        | WsChannelName::CoinExchangePrice
        | WsChannelName::CoinExchangeVolume => Some(coins),
        _ => None,
    };

    let exchanges_expected = match method {
        WsChannelName::CoinExchangePrice | WsChannelName::CoinExchangeVolume => Some(exchanges),
        _ => None,
    };

    (
        request,
        SubscriptionParams {
            id,
            channel: method,
            coins: coins_expected,
            exchanges: exchanges_expected,
        },
        expected,
    )
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

fn zip_coins_with_usd(coins: Option<Vec<String>>) -> Vec<Option<(String, String)>> {
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
    coins: Option<Vec<String>>,
    exchanges: Option<Vec<String>>,
) -> (Vec<Option<(String, String)>>, Vec<String>) {
    let pairs = zip_coins_with_usd(coins);
    let exchanges = exchanges
        .map(|v| v.to_vec())
        .unwrap_or(vec!["worker".to_string()]);

    (pairs, exchanges)
}

fn check_subscriptions(
    repositories_prepared: RepositoriesPrepared,
    subscriptions: Vec<(
        SubscriptionParams,
        Result<WsChannelSubscriptionRequest, ErrorType>,
    )>,
) -> Result<(), String> {
    for (subscription_params, subscription_request_expected) in subscriptions {
        let SubscriptionParams {
            id: sub_id_expected,
            channel: method_expected,
            coins: coins_expected,
            exchanges: exchanges_expected,
        } = subscription_params;

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

        if let Ok(subscription_request_expected) = subscription_request_expected {
            for subscription_request in subscription_requests {
                if subscription_request != subscription_request_expected {
                    return Err(format!(
                        "Error: subscription_request != subscription_request_expected. Got: {:?}, Expected: {:?}. Method: {:?}",
                        subscription_request, subscription_request_expected, method_expected,
                    ));
                }
            }
        }
    }

    Ok(())
}

fn get_all_subscription_requests(
    config: &ConfigScheme,
    channels: Vec<WsChannelName>,
    with_errors: bool,
) -> Vec<(
    String,
    SubscriptionParams,
    Result<WsChannelSubscriptionRequest, ErrorType>,
)> {
    let mut subscription_requests = Vec::new();

    for channel in channels {
        if with_errors {
            for error in ErrorType::get_by_channel(channel) {
                subscription_requests.push(make_request(config, channel, Some(error)));
            }
        } else {
            subscription_requests.push(make_request(config, channel, None));
        }
    }

    subscription_requests
}

fn get_all_subscription_requests_unzipped(
    config: &ConfigScheme,
    channels: Vec<WsChannelName>,
    with_errors: bool,
) -> (
    Vec<String>,
    Vec<SubscriptionParams>,
    Vec<Result<WsChannelSubscriptionRequest, ErrorType>>,
) {
    let mut requests = Vec::new();
    let mut params_vec = Vec::new();
    let mut expecteds = Vec::new();

    for (request, params_tuple, expected) in
        get_all_subscription_requests(config, channels, with_errors)
    {
        requests.push(request);
        params_vec.push(params_tuple);
        expecteds.push(expected);
    }

    (requests, params_vec, expecteds)
}

#[test]
#[serial]
fn test_add_ws_channels_separately() {
    let mut port = 8100;
    let mut config = ConfigScheme::default();
    let channels = WsChannelName::get_all_channels();

    for (request, params, expected) in get_all_subscription_requests(&config, channels, false) {
        config.service.ws_addr = format!("127.0.0.1:{}", port);
        port += 1;

        let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
            start_application(config.clone());

        ws_connect_and_subscribe(&config.service.ws_addr, vec![request], incoming_msg_tx);
        let res = check_subscriptions(repositories_prepared, vec![(params, expected)]);
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

    for (request, params, expected) in get_all_subscription_requests(&config, channels, true) {
        config.service.ws_addr = format!("127.0.0.1:{}", port);
        port += 1;

        let (_rx, (incoming_msg_tx, _incoming_msg_rx), config, repositories_prepared) =
            start_application(config.clone());

        ws_connect_and_subscribe(
            &config.service.ws_addr,
            vec![request.clone()],
            incoming_msg_tx,
        );
        let res = check_subscriptions(
            repositories_prepared,
            vec![(params.clone(), expected.clone())],
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

    let (requests, params, expecteds) =
        get_all_subscription_requests_unzipped(&config, channels, false);

    let expecteds = params.into_iter().zip(expecteds).collect();

    ws_connect_and_subscribe(&config.service.ws_addr, requests, incoming_msg_tx);
    let res = check_subscriptions(repositories_prepared, expecteds);
    if res.is_err() {
        panic!("Expected Ok. Got: {:?}", res);
    }
}
