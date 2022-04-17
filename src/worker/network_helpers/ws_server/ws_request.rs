use crate::worker::helper_functions::date_time_from_timestamp_sec;
use crate::worker::network_helpers::ws_server::channels::market_channels::LocalMarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::LocalWorkerChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_action::WsChannelAction;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::channels::ws_channel_unsubscribe::WsChannelUnsubscribe;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcRequest;
use crate::worker::network_helpers::ws_server::requests::ws_method_request::WsMethodRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use chrono::Utc;
use parse_duration::parse;
use serde_json::Map;

#[derive(Debug)]
pub enum WsRequest {
    Channel(WsChannelAction),
    Method(WsMethodRequest),
}

impl WsRequest {
    pub fn new(
        request: JsonRpcRequest,
        ws_answer_timeout_ms: u64,
        percent_change_interval_sec: u64,
    ) -> Result<Self, String> {
        let id = request.id.clone();
        let object = request
            .params
            .as_object()
            .ok_or("\"params\" is required and must be an object")?;
        let coins = Self::parse_vec_of_str(object, "coins");
        if let Ok(coins) = &coins {
            if coins.is_empty() {
                return Err("\"coins\" must not be empty".to_string());
            }
        }

        let frequency_ms =
            Self::parse_u64(object, "frequency_ms").unwrap_or(Ok(ws_answer_timeout_ms))?;
        if frequency_ms < ws_answer_timeout_ms {
            return Err(format!(
                "\"frequency_ms\" must be not less than {}",
                ws_answer_timeout_ms,
            ));
        }

        let percent_change_interval_sec = object
            .get("percent_change_interval")
            .map(|v| {
                v.as_str()
                    .map(|v| {
                        parse(v)
                            .map(|v| v.as_secs())
                            .map_err(|_| "\"percent_change_interval\": invalid value")
                    })
                    .ok_or("\"percent_change_interval\" must be a string")?
            })
            .unwrap_or(Ok(percent_change_interval_sec));
        if percent_change_interval_sec < Ok(1) {
            return Err("\"percent_change_interval\" must be at least 1 second".to_string());
        }

        let interval_sec = object
            .get("interval")
            .ok_or("\"interval\" is required")
            .map(|v| v.as_str().ok_or("\"interval\" must be a string"))
            .map(|v| {
                v.map(|v| {
                    parse(v)
                        .map(|v| v.as_secs())
                        .map_err(|_| "\"interval\": invalid value")
                })
            });
        if interval_sec < Ok(Ok(Ok(1))) {
            return Err("\"interval\" must be at least 1 second".to_string());
        }

        match request.method {
            WsChannelName::AvailableCoins => {
                let res = WsMethodRequest::AvailableCoins { id };

                Ok(Self::Method(res))
            }
            WsChannelName::IndexPrice | WsChannelName::IndexPriceCandles => {
                let res = match request.method {
                    WsChannelName::IndexPrice => LocalWorkerChannels::IndexPrice {
                        id,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    WsChannelName::IndexPriceCandles => LocalWorkerChannels::IndexPriceCandles {
                        id,
                        frequency_ms,
                        interval_sec: interval_sec???,
                    },
                    _ => unreachable!(),
                };

                Ok(Self::Channel(WsChannelAction::Subscribe(
                    WsChannelSubscriptionRequest::Worker(res),
                )))
            }
            WsChannelName::CoinAveragePrice | WsChannelName::CoinAveragePriceCandles => {
                let coins = coins?;

                let res = match request.method {
                    WsChannelName::CoinAveragePrice => LocalWorkerChannels::CoinAveragePrice {
                        id,
                        coins,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    WsChannelName::CoinAveragePriceCandles => {
                        LocalWorkerChannels::CoinAveragePriceCandles {
                            id,
                            coins,
                            frequency_ms,
                            interval_sec: interval_sec???,
                        }
                    }
                    _ => unreachable!(),
                };

                Ok(Self::Channel(WsChannelAction::Subscribe(
                    WsChannelSubscriptionRequest::Worker(res),
                )))
            }
            WsChannelName::CoinExchangePrice | WsChannelName::CoinExchangeVolume => {
                let coins = coins?;
                let exchanges = Self::parse_vec_of_str(object, "exchanges")?;
                if exchanges.is_empty() {
                    return Err("\"exchanges\" must not be empty".to_string());
                }

                let res = match request.method {
                    WsChannelName::CoinExchangePrice => LocalMarketChannels::CoinExchangePrice {
                        id,
                        coins,
                        exchanges,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    WsChannelName::CoinExchangeVolume => LocalMarketChannels::CoinExchangeVolume {
                        id,
                        coins,
                        exchanges,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    _ => unreachable!(),
                };

                Ok(Self::Channel(WsChannelAction::Subscribe(
                    WsChannelSubscriptionRequest::Market(res),
                )))
            }
            WsChannelName::Unsubscribe => Ok(Self::Channel(WsChannelAction::Unsubscribe(
                WsChannelUnsubscribe { id },
            ))),
            WsChannelName::IndexPriceHistorical
            | WsChannelName::IndexPriceCandlesHistorical
            | WsChannelName::CoinAveragePriceHistorical
            | WsChannelName::CoinAveragePriceCandlesHistorical => {
                let coin = object.get("coin").ok_or("\"coin\" is required");
                let interval_sec = interval_sec???;

                let from = Self::parse_u64(object, "from").ok_or("\"from\" is required")??;
                let from = date_time_from_timestamp_sec(from);

                let to = Self::parse_u64(object, "to").transpose()?;
                let to = to
                    .map(date_time_from_timestamp_sec)
                    .unwrap_or_else(Utc::now);

                let res = match request.method {
                    WsChannelName::IndexPriceHistorical => WsMethodRequest::IndexPriceHistorical {
                        id,
                        interval_sec,
                        from,
                        to,
                    },
                    WsChannelName::IndexPriceCandlesHistorical => {
                        WsMethodRequest::IndexPriceCandlesHistorical {
                            id,
                            interval_sec,
                            from,
                            to,
                        }
                    }
                    WsChannelName::CoinAveragePriceHistorical => {
                        WsMethodRequest::CoinAveragePriceHistorical {
                            id,
                            coin: coin?.as_str().ok_or("\"coin\" must be string")?.to_string(),
                            interval_sec,
                            from,
                            to,
                        }
                    }
                    WsChannelName::CoinAveragePriceCandlesHistorical => {
                        WsMethodRequest::CoinAveragePriceCandlesHistorical {
                            id,
                            coin: coin?.as_str().ok_or("\"coin\" must be string")?.to_string(),
                            interval_sec,
                            from,
                            to,
                        }
                    }
                    _ => unreachable!(),
                };

                Ok(Self::Method(res))
            }
        }
    }

    fn parse_vec_of_str(
        object: &Map<String, serde_json::Value>,
        key: &str,
    ) -> Result<Vec<String>, String> {
        let mut res_items = Vec::new();

        let items = object.get(key).ok_or(format!("\"{}\" is required", key))?;
        let items = items
            .as_array()
            .ok_or(format!("\"{}\" must be an array", key))?;

        for item in items {
            res_items.push(
                item.as_str()
                    .ok_or(format!("\"{}\" must be an array of string", key))?
                    .to_string(),
            );
        }

        Ok(res_items)
    }

    fn parse_u64(
        object: &Map<String, serde_json::Value>,
        key: &str,
    ) -> Option<Result<u64, String>> {
        object
            .get(key)
            .map(|v| v.as_u64().ok_or(format!("\"{}\" must be an uint", key)))
    }
}
