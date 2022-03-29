use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MarketChannels {
    CoinExchangePrice {
        id: JsonRpcId,
        coins: Vec<String>,
        exchanges: Vec<String>,
        frequency_ms: Option<u64>,
        percent_change_interval_sec: Option<u64>,
    },
    CoinExchangeVolume {
        id: JsonRpcId,
        coins: Vec<String>,
        exchanges: Vec<String>,
        frequency_ms: Option<u64>,
        percent_change_interval_sec: Option<u64>,
    },
}
