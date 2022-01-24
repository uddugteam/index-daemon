use crate::worker::network_helpers::ws_server::json_rpc_messages::{JsonRpcId, JsonRpcRequest};

#[derive(Debug)]
pub enum WsChannelRequest {
    CoinAveragePrice {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        frequency_ms: u64,
    },
    Unsubscribe {
        id: Option<JsonRpcId>,
        method: String,
    },
}

impl WsChannelRequest {
    pub fn get_id(&self) -> Option<JsonRpcId> {
        match self {
            WsChannelRequest::CoinAveragePrice { id, .. } => id,
            WsChannelRequest::Unsubscribe { id, .. } => id,
        }
        .clone()
    }

    pub fn get_method(&self) -> String {
        match self {
            WsChannelRequest::CoinAveragePrice { .. } => "coin_average_price".to_string(),
            WsChannelRequest::Unsubscribe { method, .. } => method.to_string(),
        }
    }

    pub fn get_frequency_ms(&self) -> u64 {
        let frequency_ms = match self {
            WsChannelRequest::CoinAveragePrice { frequency_ms, .. } => *frequency_ms,
            WsChannelRequest::Unsubscribe { .. } => {
                panic!("Unexpected request.")
            }
        };

        frequency_ms
    }

    pub fn set_frequency_ms(&mut self, new_frequency_ms: u64) {
        match self {
            WsChannelRequest::CoinAveragePrice { frequency_ms, .. } => {
                *frequency_ms = new_frequency_ms;
            }
            WsChannelRequest::Unsubscribe { .. } => {
                panic!("Unexpected request.")
            }
        }
    }

    pub fn get_coins(&self) -> &Vec<String> {
        let coins = match self {
            WsChannelRequest::CoinAveragePrice { coins, .. } => coins,
            WsChannelRequest::Unsubscribe { .. } => {
                panic!("Unexpected request.")
            }
        };

        coins
    }
}

impl TryFrom<&JsonRpcRequest> for WsChannelRequest {
    type Error = String;

    fn try_from(request: &JsonRpcRequest) -> Result<Self, Self::Error> {
        let e = "Wrong params.";
        let id = request.id.clone();
        let object = request.params.as_object().ok_or(e)?;

        match request.method.as_str() {
            "coin_average_price" => {
                let coins_json = object.get("coins").ok_or(e)?;
                let coins_json = coins_json.as_array().ok_or(e)?;
                let mut coins = Vec::new();
                for coin in coins_json {
                    coins.push(coin.as_str().ok_or(e)?.to_string());
                }

                let frequency_ms = object.get("frequency_ms").ok_or(e)?;
                let frequency_ms = frequency_ms.as_u64().ok_or(e)?;

                Ok(Self::CoinAveragePrice {
                    id,
                    coins,
                    frequency_ms,
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
