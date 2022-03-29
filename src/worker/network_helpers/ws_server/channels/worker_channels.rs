use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WorkerChannels {
    IndexPrice {
        id: JsonRpcId,
        frequency_ms: u64,
        percent_change_interval_sec: Option<u64>,
    },
    IndexPriceCandles {
        id: JsonRpcId,
        frequency_ms: u64,
        interval_sec: u64,
    },
    CoinAveragePrice {
        id: JsonRpcId,
        coins: Vec<String>,
        frequency_ms: u64,
        percent_change_interval_sec: Option<u64>,
    },
    CoinAveragePriceCandles {
        id: JsonRpcId,
        coins: Vec<String>,
        frequency_ms: u64,
        interval_sec: u64,
    },
}
