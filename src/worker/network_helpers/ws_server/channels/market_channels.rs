use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone)]
pub enum MarketChannels {
    CoinExchangePrice {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        exchanges: Vec<String>,
        frequency_ms: Option<u64>,
    },
    CoinExchangeVolume {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        exchanges: Vec<String>,
        frequency_ms: Option<u64>,
    },
}

impl MarketChannels {
    pub fn get_exchanges(&self) -> &[String] {
        match self {
            MarketChannels::CoinExchangePrice { exchanges, .. }
            | MarketChannels::CoinExchangeVolume { exchanges, .. } => exchanges,
        }
    }
}
