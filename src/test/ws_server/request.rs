use crate::config_scheme::config_scheme::ConfigScheme;
use crate::test::ws_server::error_type::{ErrorType, Field};
use crate::test::ws_server::helper_functions::SubscriptionParams;
use crate::worker::network_helpers::ws_server::channels::market_channels::LocalMarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::LocalWorkerChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_action::WsChannelAction;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_request::WsRequest;
use parse_duration::parse;
use serde_json::json;
use uuid::Uuid;

pub struct Requests(pub Vec<Request>);

impl Requests {
    pub fn make_all(
        config: &ConfigScheme,
        channels: &[WsChannelName],
        with_errors: bool,
        sub_id: Option<JsonRpcId>,
    ) -> Self {
        let mut requests = Vec::new();

        for &channel in channels {
            if with_errors {
                let errors = ErrorType::get_by_channel(channel);

                if !errors.is_empty() {
                    for error in errors {
                        requests.push(Request::make(config, channel, Some(error), sub_id.clone()));
                    }
                } else {
                    panic!("No errors exists for: {:?}", channel);
                }
            } else {
                requests.push(Request::make(config, channel, None, sub_id.clone()));
            }
        }

        Self(requests)
    }

    pub fn unzip(self) -> RequestsUnzipped {
        let mut requests = Vec::new();
        let mut params_vec = Vec::new();
        let mut expecteds = Vec::new();

        for request in self.0 {
            let Request {
                request,
                params_item,
                expected,
            } = request;

            requests.push(request);
            params_vec.push(params_item);
            expecteds.push(expected);
        }

        RequestsUnzipped {
            requests,
            params_vec,
            expecteds,
        }
    }
}

pub struct Request {
    pub request: String,
    pub params_item: SubscriptionParams,
    pub expected: Result<WsRequest, ErrorType>,
}

impl Request {
    pub fn make(
        config: &ConfigScheme,
        method: WsChannelName,
        error: Option<ErrorType>,
        sub_id: Option<JsonRpcId>,
    ) -> Self {
        let id = sub_id.unwrap_or(JsonRpcId::Str(Uuid::new_v4().to_string()));
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

        Self::spoil_request(config, &mut request, error);

        let request = serde_json::to_string(&request).unwrap();

        let expected = if let Some(error) = error {
            Err(error)
        } else {
            let id = id.clone();
            let coins = coins.clone();
            let exchanges = exchanges.clone();
            let percent_change_interval_sec = parse(&percent_change_interval).unwrap().as_secs();
            let interval_sec = parse(&interval).unwrap().as_secs();

            let expected = match method {
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
            };

            let expected = expected.map(WsChannelAction::Subscribe);
            let expected = expected.map(WsRequest::Channel);

            expected
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

        Self {
            request,
            params_item: SubscriptionParams {
                id,
                channel: method,
                coins: coins_expected,
                exchanges: exchanges_expected,
            },
            expected,
        }
    }

    fn spoil_request(
        config: &ConfigScheme,
        request: &mut serde_json::Value,
        error: Option<ErrorType>,
    ) {
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
}

pub struct RequestsUnzipped {
    pub requests: Vec<String>,
    pub params_vec: Vec<SubscriptionParams>,
    pub expecteds: Vec<Result<WsRequest, ErrorType>>,
}