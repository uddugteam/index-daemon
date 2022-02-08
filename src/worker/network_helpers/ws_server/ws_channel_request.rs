use crate::worker::network_helpers::ws_server::jsonrpc_messages::{JsonRpcId, JsonRpcRequest};
use serde_json::Map;

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
}
impl Interval {
    fn into_seconds(self) -> u64 {
        match self {
            Self::Second => 1,
            Self::Minute => 60,
            Self::Hour => 3600,
            Self::Day => 86400,
            Self::Week => 604800,
            Self::Month => 2592000,
        }
    }

    pub fn into_millis(self) -> u64 {
        self.into_seconds() * 1000
    }
}

#[derive(Debug, Clone)]
pub enum WsChannelRequest {
    CoinAveragePrice {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        frequency_ms: u64,
    },
    CoinExchangePrice {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        exchanges: Vec<String>,
        frequency_ms: u64,
    },
    CoinExchangeVolume {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        exchanges: Vec<String>,
        frequency_ms: u64,
    },
    CoinAveragePriceHistorical {
        id: Option<JsonRpcId>,
        coin: String,
        interval: Interval,
        from: u64,
        to: u64,
    },
    Unsubscribe {
        id: Option<JsonRpcId>,
        method: String,
    },
}

impl WsChannelRequest {
    pub fn get_id(&self) -> Option<JsonRpcId> {
        match self {
            Self::CoinAveragePrice { id, .. }
            | Self::CoinExchangePrice { id, .. }
            | Self::CoinExchangeVolume { id, .. }
            | Self::CoinAveragePriceHistorical { id, .. }
            | Self::Unsubscribe { id, .. } => id.clone(),
        }
    }

    pub fn get_method(&self) -> String {
        match self {
            Self::CoinAveragePrice { .. } => "coin_average_price".to_string(),
            Self::CoinExchangePrice { .. } => "coin_exchange_price".to_string(),
            Self::CoinExchangeVolume { .. } => "coin_exchange_volume".to_string(),
            Self::CoinAveragePriceHistorical { .. } => "coin_average_price_historical".to_string(),
            Self::Unsubscribe { method, .. } => method.to_string(),
        }
    }

    pub fn get_frequency_ms(&self) -> u64 {
        match self {
            Self::CoinAveragePrice { frequency_ms, .. }
            | Self::CoinExchangePrice { frequency_ms, .. }
            | Self::CoinExchangeVolume { frequency_ms, .. } => *frequency_ms,
            Self::CoinAveragePriceHistorical { .. } | Self::Unsubscribe { .. } => {
                unreachable!();
            }
        }
    }

    pub fn set_frequency_ms(&mut self, new_frequency_ms: u64) {
        match self {
            Self::CoinAveragePrice { frequency_ms, .. }
            | Self::CoinExchangePrice { frequency_ms, .. }
            | Self::CoinExchangeVolume { frequency_ms, .. } => {
                *frequency_ms = new_frequency_ms;
            }
            Self::CoinAveragePriceHistorical { .. } | Self::Unsubscribe { .. } => {
                unreachable!();
            }
        }
    }

    pub fn get_coins(&self) -> &[String] {
        match self {
            Self::CoinAveragePrice { coins, .. }
            | Self::CoinExchangePrice { coins, .. }
            | Self::CoinExchangeVolume { coins, .. } => coins,
            Self::CoinAveragePriceHistorical { .. } | Self::Unsubscribe { .. } => {
                unreachable!();
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

    fn parse_u64(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
        object.get(key)?.as_u64()
    }
}

impl TryFrom<JsonRpcRequest> for WsChannelRequest {
    type Error = String;

    fn try_from(request: JsonRpcRequest) -> Result<Self, Self::Error> {
        let e = "Wrong params.";
        let id = request.id.clone();
        let object = request.params.as_object().ok_or(e)?;
        let coins = Self::parse_vec_of_str(object, "coins").ok_or(e);
        let frequency_ms = Self::parse_u64(object, "frequency_ms").ok_or(e);

        match request.method.as_str() {
            "coin_average_price" => {
                let coins = coins?;
                let frequency_ms = frequency_ms?;

                Ok(Self::CoinAveragePrice {
                    id,
                    coins,
                    frequency_ms,
                })
            }
            "coin_exchange_price" | "coin_exchange_volume" => {
                let coins = coins?;
                let exchanges = Self::parse_vec_of_str(object, "exchanges").ok_or(e)?;
                let frequency_ms = frequency_ms?;

                match request.method.as_str() {
                    "coin_exchange_price" => Ok(Self::CoinExchangePrice {
                        id,
                        coins,
                        exchanges,
                        frequency_ms,
                    }),
                    "coin_exchange_volume" => Ok(Self::CoinExchangeVolume {
                        id,
                        coins,
                        exchanges,
                        frequency_ms,
                    }),
                    _ => unreachable!(),
                }
            }
            "coin_average_price_historical" => {
                let coin = object.get("coin").ok_or(e)?.as_str().ok_or(e)?.to_string();
                let interval = object.get("interval").cloned().ok_or(e)?;
                let interval: Interval = serde_json::from_value(interval).map_err(|_| e)?;
                let from = Self::parse_u64(object, "from").ok_or(e)?;
                let to = Self::parse_u64(object, "to").ok_or(e)?;

                Ok(Self::CoinAveragePriceHistorical {
                    id,
                    coin,
                    interval,
                    from,
                    to,
                })
            }
            "unsubscribe" => {
                let method = object.get("method").ok_or(e)?;
                let method = method.as_str().ok_or(e)?.to_string();

                Ok(Self::Unsubscribe { id, method })
            }
            _ => Err("Wrong method.".to_string()),
        }
    }
}
