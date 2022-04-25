use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
use crate::config_scheme::storage::Storage;
use crate::graceful_shutdown::GracefulShutdown;
use crate::test::ws_server::error_type::ErrorType;
use crate::test::ws_server::ws_client_for_testing::WsClientForTesting;
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use crate::worker::network_helpers::ws_server::channels::ws_channel_action::WsChannelAction;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channel_response::WsChannelResponse;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_request::WsRequest;
use crate::worker::worker::start_worker;
use futures::Future;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{sleep, Duration};

pub type SubscriptionsExpected = (
    SubscriptionParams,
    Result<WsChannelSubscriptionRequest, ErrorType>,
);

type Received = (
    HashMap<JsonRpcId, Result<WsChannelName, String>>,
    Vec<(Option<WsChannelName>, WsChannelResponse)>,
);

#[derive(Debug, Clone)]
pub struct SubscriptionParams {
    pub id: JsonRpcId,
    pub channel: WsChannelName,
    pub coins: Option<Vec<String>>,
    pub exchanges: Option<Vec<String>>,
}

pub fn start_application(
    mut config: ConfigScheme,
) -> (
    impl Future<Output = ()>,
    (UnboundedSender<String>, UnboundedReceiver<String>),
    ConfigScheme,
    RepositoriesPrepared,
) {
    let graceful_shutdown = GracefulShutdown::new();

    config.service.ws = true;
    config.service.storage = Some(Storage::default());
    config.market.channels = vec![
        ExternalMarketChannels::Ticker,
        ExternalMarketChannels::Trades,
    ];

    let repositories_prepared = RepositoriesPrepared::make(&config);

    let worker_future = start_worker(
        config.clone(),
        repositories_prepared.clone(),
        graceful_shutdown,
    );

    (
        worker_future,
        mpsc::unbounded_channel(),
        config,
        repositories_prepared,
    )
}

pub fn make_unsub_request(sub_id: JsonRpcId) -> String {
    let request = json!({
        "id": sub_id,
        "jsonrpc": "2.0",
        "method": "unsubscribe",
        "params": {}
    });

    serde_json::to_string(&request).unwrap()
}

pub async fn ws_connect_and_send(
    ws_addr: String,
    outgoing_messages: Vec<String>,
    incoming_msg_tx: UnboundedSender<String>,
) {
    let uri = "ws://".to_string() + &ws_addr;

    // Give Websocket server time to start
    sleep(Duration::from_millis(3000)).await;

    let ws_client = WsClientForTesting::new(uri, outgoing_messages, incoming_msg_tx);
    ws_client.run().await;
}

pub fn zip_coins_with_usd(coins: Option<Vec<String>>) -> Vec<Option<(String, String)>> {
    coins
        .map(|v| {
            v.iter()
                .map(|coin| (coin.to_string(), "USD".to_string()))
                .map(Some)
                .collect()
        })
        .unwrap_or(vec![None])
}

pub async fn get_subscription_requests(
    repositories_prepared: &RepositoriesPrepared,
    sub_id: &JsonRpcId,
    method: WsChannelName,
    pairs: &[Option<(String, String)>],
    exchanges: &[MarketValueOwner],
) -> Vec<WsChannelSubscriptionRequest> {
    let mut subscription_requests = Vec::new();

    for pair in pairs {
        for exchange in exchanges {
            let key = (exchange.clone(), method.get_market_value(), pair.clone());
            let ws_channels = repositories_prepared
                .ws_channels_holder
                .get(&key)
                .unwrap()
                .read()
                .await;

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

pub fn prepare_some_params(
    coins: Option<Vec<String>>,
    exchanges: Option<Vec<String>>,
) -> (Vec<Option<(String, String)>>, Vec<MarketValueOwner>) {
    let pairs = zip_coins_with_usd(coins);
    let exchanges = exchanges
        .map(|v| v.into_iter().map(MarketValueOwner::Market).collect())
        .unwrap_or(vec![MarketValueOwner::Worker]);

    (pairs, exchanges)
}

pub fn extract_subscription_request(request: WsRequest) -> Option<WsChannelSubscriptionRequest> {
    match request {
        WsRequest::Channel(WsChannelAction::Subscribe(request)) => Some(request),
        _ => None,
    }
}

pub fn parse_succ_sub_or_err_response(
    json_string: &str,
    expected: &HashMap<JsonRpcId, WsChannelName>,
) -> Result<(WsChannelResponse, Option<WsChannelName>), String> {
    let mut json = serde_json::from_str::<serde_json::Value>(json_string)
        .map_err(|_| "Error parsing response json.".to_string())?;
    let object = json
        .as_object_mut()
        .ok_or("Response msg must be an object.".to_string())?;

    let id = object
        .get("id")
        .ok_or("Response object must have key \"id\"".to_string())?
        .to_string();
    let id = serde_json::from_str::<JsonRpcId>(&id)
        .map_err(|_| "Error parsing field \"id\"".to_string());
    let method_expected = if let Ok(id) = id {
        expected
            .get(&id)
            .ok_or(format!("Got unexpected id: {:#?}", id))
            .ok()
            .cloned()
    } else {
        None
    };

    let result = object
        .get_mut("result")
        .ok_or("Response object must have key \"result\"".to_string())?
        .as_object_mut()
        .ok_or("Field \"result\" must be an object.".to_string())?;

    let method = result
        .get("method")
        .ok_or("Params object must have key \"method\"".to_string());

    let method = if let Ok(method) = method {
        let method = method.to_string();

        let method = serde_json::from_str::<WsChannelName>(&method)
            .map_err(|_| "Error parsing field \"method\"".to_string())?;

        if let Some(method_expected) = method_expected {
            if method != method_expected {
                return Err(format!(
                    "Got unexpected method. Got: {:#?}. Expected: {:#?}.",
                    method, method_expected,
                ));
            }
        }

        Some(method)
    } else {
        None
    };

    if result.contains_key("code") {
        // Err

        result.insert(
            "method".to_string(),
            serde_json::Value::String("err".to_string()),
        );
    } else {
        // SuccSub

        result.insert(
            "method".to_string(),
            serde_json::Value::String("succ_sub".to_string()),
        );
    }

    let json_string_mod = json.to_string();

    serde_json::from_str::<WsChannelResponse>(&json_string_mod)
        .map(|v| (v, method))
        .map_err(|e| e.to_string())
}

pub async fn check_subscriptions(
    repositories_prepared: RepositoriesPrepared,
    subscriptions: Vec<SubscriptionsExpected>,
) -> Result<Vec<WsChannelName>, String> {
    // Give Websocket server time to process request
    sleep(Duration::from_millis(10000)).await;

    let channel_names = subscriptions.iter().map(|v| v.0.channel).collect();

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
        )
        .await;

        if subscription_requests.len() != expected_len {
            return Err(format!(
                "Error: subscription_requests.len() != expected_len. Got: {}, Expected: {}. Method: {:#?}",
                subscription_requests.len(), expected_len, method_expected,
            ));
        }

        if let Ok(subscription_request_expected) = subscription_request_expected {
            for subscription_request in subscription_requests {
                if subscription_request != subscription_request_expected {
                    return Err(format!(
                        "Error: subscription_request != subscription_request_expected. Got: {:#?}, Expected: {:#?}. Method: {:#?}",
                        subscription_request, subscription_request_expected, method_expected,
                    ));
                }
            }
        }
    }

    Ok(channel_names)
}

pub async fn check_incoming_messages(
    mut incoming_msg_rx: UnboundedReceiver<String>,
    expected: HashMap<JsonRpcId, WsChannelName>,
) -> Received {
    let mut res = HashMap::new();
    let mut no_id = Vec::new();

    let start = Instant::now();
    let minutes = 4;
    while start.elapsed().as_secs() < minutes * 60 {
        if let Ok(incoming_msg) = incoming_msg_rx.try_recv() {
            let (response, method) = match serde_json::from_str::<WsChannelResponse>(&incoming_msg)
            {
                Ok(response) => {
                    let method = response.result.get_method();

                    (response, method)
                }
                Err(_) => parse_succ_sub_or_err_response(&incoming_msg, &expected)
                    .map_err(|e| {
                        format!(
                            "Parse WsChannelResponse error. Response: {}. Error: {}",
                            incoming_msg, e
                        )
                    })
                    .unwrap(),
            };

            if let Some(id) = response.id.clone() {
                match response.result {
                    WsChannelResponsePayload::SuccSub { .. } => {
                        // Ignore
                    }
                    WsChannelResponsePayload::Err { message, .. } => {
                        res.insert(id, Err(format!("Received error message: {}", message)));
                    }
                    _ => {
                        if expected.contains_key(&id) {
                            let method = response.result.get_method().unwrap();

                            res.entry(id).or_insert(Ok(method));
                        } else {
                            res.insert(
                                id,
                                Err(format!("Got unexpected id. Response: {:#?}", response)),
                            );
                        }
                    }
                }
            } else {
                no_id.push((method, response));
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }

    (res, no_id)
}
