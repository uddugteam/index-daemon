#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "method", content = "params")]
/// TODO: Store request id
pub enum WsChannelRequest {
    CoinAveragePrice {
        coins: Vec<String>,
        frequency_ms: u64,
    },

    Unsubscribe,
}

impl WsChannelRequest {
    pub fn get_frequency_ms(&self) -> u64 {
        let frequency_ms = match self {
            WsChannelRequest::CoinAveragePrice { frequency_ms, .. } => *frequency_ms,
            WsChannelRequest::Unsubscribe => {
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
            WsChannelRequest::Unsubscribe => {
                panic!("Unexpected request.")
            }
        }
    }

    pub fn get_coins(&self) -> &Vec<String> {
        let coins = match self {
            WsChannelRequest::CoinAveragePrice { coins, .. } => coins,
            WsChannelRequest::Unsubscribe => {
                panic!("Unexpected request.")
            }
        };

        coins
    }
}
