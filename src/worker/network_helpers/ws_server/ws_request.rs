use crate::worker::network_helpers::ws_server::channels::market_channels::MarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::WorkerChannels;
use crate::worker::network_helpers::ws_server::channels::ws_channel_action::WsChannelAction;
use crate::worker::network_helpers::ws_server::channels::ws_channel_subscription_request::WsChannelSubscriptionRequest;
use crate::worker::network_helpers::ws_server::channels::ws_channel_unsubscribe::WsChannelUnsubscribe;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcRequest;
use crate::worker::network_helpers::ws_server::requests::ws_method_request::WsMethodRequest;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use parse_duration::parse;
use serde_json::Map;

#[derive(Debug)]
pub enum WsRequest {
    Channel(WsChannelAction),
    Method(WsMethodRequest),
}

impl WsRequest {
    pub fn new(request: JsonRpcRequest, ws_answer_timeout_ms: u64) -> Result<Self, String> {
        let e = "Wrong params.";
        let id = request.id.clone();
        let object = request.params.as_object().ok_or(e)?;
        let coins = Self::parse_vec_of_str(object, "coins").ok_or(e);

        let frequency_ms = Self::parse_u64(object, "frequency_ms").unwrap_or(ws_answer_timeout_ms);
        if frequency_ms < ws_answer_timeout_ms {
            return Err(format!(
                "\"frequency_ms\" must be not less than {}",
                ws_answer_timeout_ms,
            ));
        }

        let percent_change_interval_sec = object
            .get("percent_change_interval")
            .map(|v| v.as_str().map(|v| parse(v).unwrap().as_secs()).ok_or(e));
        let percent_change_interval_sec = Self::swap(percent_change_interval_sec);
        let interval_sec = object
            .get("interval")
            .ok_or(e)
            .map(|v| v.as_str().ok_or(e))
            .map(|v| v.map(|v| parse(v).map(|v| v.as_secs()).map_err(|_| e)));

        match request.method {
            WsChannelName::AvailableCoins => {
                let res = WsMethodRequest::AvailableCoins { id };

                Ok(Self::Method(res))
            }
            WsChannelName::IndexPrice | WsChannelName::IndexPriceCandles => {
                let res = match request.method {
                    WsChannelName::IndexPrice => WorkerChannels::IndexPrice {
                        id,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    WsChannelName::IndexPriceCandles => WorkerChannels::IndexPriceCandles {
                        id,
                        frequency_ms,
                        interval_sec: interval_sec???,
                    },
                    _ => unreachable!(),
                };

                Ok(Self::Channel(WsChannelAction::Subscribe(
                    WsChannelSubscriptionRequest::WorkerChannels(res),
                )))
            }
            WsChannelName::CoinAveragePrice | WsChannelName::CoinAveragePriceCandles => {
                let coins = coins?;

                let res = match request.method {
                    WsChannelName::CoinAveragePrice => WorkerChannels::CoinAveragePrice {
                        id,
                        coins,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    WsChannelName::CoinAveragePriceCandles => {
                        WorkerChannels::CoinAveragePriceCandles {
                            id,
                            coins,
                            frequency_ms,
                            interval_sec: interval_sec???,
                        }
                    }
                    _ => unreachable!(),
                };

                Ok(Self::Channel(WsChannelAction::Subscribe(
                    WsChannelSubscriptionRequest::WorkerChannels(res),
                )))
            }
            WsChannelName::CoinExchangePrice | WsChannelName::CoinExchangeVolume => {
                let coins = coins?;
                let exchanges = Self::parse_vec_of_str(object, "exchanges").ok_or(e)?;

                let res = match request.method {
                    WsChannelName::CoinExchangePrice => MarketChannels::CoinExchangePrice {
                        id,
                        coins,
                        exchanges,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    WsChannelName::CoinExchangeVolume => MarketChannels::CoinExchangeVolume {
                        id,
                        coins,
                        exchanges,
                        frequency_ms,
                        percent_change_interval_sec: percent_change_interval_sec?,
                    },
                    _ => unreachable!(),
                };

                Ok(Self::Channel(WsChannelAction::Subscribe(
                    WsChannelSubscriptionRequest::MarketChannels(res),
                )))
            }
            WsChannelName::Unsubscribe => Ok(Self::Channel(WsChannelAction::Unsubscribe(
                WsChannelUnsubscribe { id },
            ))),
            WsChannelName::IndexPriceHistorical
            | WsChannelName::IndexPriceCandlesHistorical
            | WsChannelName::CoinAveragePriceHistorical
            | WsChannelName::CoinAveragePriceCandlesHistorical => {
                let coin = object.get("coin").ok_or(e);
                let interval_sec = interval_sec???;
                let from = Self::parse_u64(object, "from").ok_or(e)?;
                let to = Self::parse_u64(object, "to").ok_or(e).ok();

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
                            coin: coin?.as_str().ok_or(e)?.to_string(),
                            interval_sec,
                            from,
                            to,
                        }
                    }
                    WsChannelName::CoinAveragePriceCandlesHistorical => {
                        WsMethodRequest::CoinAveragePriceCandlesHistorical {
                            id,
                            coin: coin?.as_str().ok_or(e)?.to_string(),
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

    fn parse_vec_of_str(object: &Map<String, serde_json::Value>, key: &str) -> Option<Vec<String>> {
        let mut items = Vec::new();
        for item in object.get(key)?.as_array()? {
            items.push(item.as_str()?.to_string());
        }

        Some(items)
    }

    fn parse_u64(object: &Map<String, serde_json::Value>, key: &str) -> Option<u64> {
        object.get(key)?.as_u64()
    }

    fn swap<T, E>(value: Option<Result<T, E>>) -> Result<Option<T>, E> {
        match value {
            Some(value) => match value {
                Ok(value) => Ok(Some(value)),
                Err(e) => Err(e),
            },
            None => Ok(None),
        }
    }
}
