use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone)]
pub enum WorkerChannels {
    IndexPrice {
        id: JsonRpcId,
        frequency_ms: Option<u64>,
        percent_change_interval_sec: Option<u64>,
    },
    IndexPriceCandles {
        id: JsonRpcId,
        frequency_ms: Option<u64>,
        interval: Interval,
    },
    CoinAveragePrice {
        id: JsonRpcId,
        coins: Vec<String>,
        frequency_ms: Option<u64>,
        percent_change_interval_sec: Option<u64>,
    },
    CoinAveragePriceCandles {
        id: JsonRpcId,
        coins: Vec<String>,
        frequency_ms: Option<u64>,
        interval: Interval,
    },
}
