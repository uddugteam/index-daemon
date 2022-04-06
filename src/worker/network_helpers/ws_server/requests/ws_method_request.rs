use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum WsMethodRequest {
    AvailableCoins {
        id: JsonRpcId,
    },
    IndexPriceHistorical {
        id: JsonRpcId,
        interval_sec: u64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    IndexPriceCandlesHistorical {
        id: JsonRpcId,
        interval_sec: u64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    CoinAveragePriceHistorical {
        id: JsonRpcId,
        coin: String,
        interval_sec: u64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    CoinAveragePriceCandlesHistorical {
        id: JsonRpcId,
        coin: String,
        interval_sec: u64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
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
