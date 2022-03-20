use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Debug, Clone)]
pub enum WsMethodRequest {
    AvailableCoins {
        id: Option<JsonRpcId>,
    },
    IndexPriceHistorical {
        id: Option<JsonRpcId>,
        interval: Interval,
        from: u64,
        to: Option<u64>,
    },
    IndexPriceCandlesHistorical {
        id: Option<JsonRpcId>,
        interval: Interval,
        from: u64,
        to: Option<u64>,
    },
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

impl WsMethodRequest {
    pub fn get_method(&self) -> WsChannelName {
        match self {
            Self::AvailableCoins { .. } => WsChannelName::AvailableCoins,
            Self::IndexPriceHistorical { .. } => WsChannelName::IndexPriceHistorical,
            Self::IndexPriceCandlesHistorical { .. } => WsChannelName::IndexPriceCandlesHistorical,
            Self::CoinAveragePriceHistorical { .. } => WsChannelName::CoinAveragePriceHistorical,
            Self::CoinAveragePriceCandlesHistorical { .. } => {
                WsChannelName::CoinAveragePriceCandlesHistorical
            }
        }
    }
}
