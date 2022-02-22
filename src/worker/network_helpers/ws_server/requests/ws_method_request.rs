use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;

#[derive(Debug, Clone)]
pub enum WsMethodRequest {
    CoinAveragePriceHistorical {
        id: Option<JsonRpcId>,
        coin: String,
        interval: Interval,
        from: u64,
        to: Option<u64>,
    },
    CoinAveragePriceCandlesHistorical {
        id: Option<JsonRpcId>,
        coin: String,
        interval: Interval,
        from: u64,
        to: Option<u64>,
    },
}
