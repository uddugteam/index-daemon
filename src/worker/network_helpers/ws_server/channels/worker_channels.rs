use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::jsonrpc_messages::JsonRpcId;

#[derive(Debug, Clone)]
pub enum WorkerChannels {
    CoinAveragePrice {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        frequency_ms: u64,
    },
    CoinAveragePriceCandles {
        id: Option<JsonRpcId>,
        coins: Vec<String>,
        frequency_ms: u64,
        interval: Interval,
    },
}
