use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone)]
pub enum WorkerChannels {
    IndexPrice {
        id: Option<JsonRpcId>,
        frequency_ms: Option<u64>,
    },
    IndexPriceCandles {
        id: Option<JsonRpcId>,
        frequency_ms: Option<u64>,
        interval: Interval,
    },
    CoinAveragePrice {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        frequency_ms: Option<u64>,
    },
    CoinAveragePriceCandles {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        frequency_ms: Option<u64>,
        interval: Interval,
    },
}
