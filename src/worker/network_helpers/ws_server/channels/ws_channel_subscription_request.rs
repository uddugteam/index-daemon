use crate::worker::network_helpers::ws_server::channels::market_channels::MarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::WorkerChannels;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Debug, Clone)]
pub enum WsChannelSubscriptionRequest {
    WorkerChannels(WorkerChannels),
    MarketChannels(MarketChannels),
}

impl WsChannelSubscriptionRequest {
    pub fn get_id(&self) -> Option<JsonRpcId> {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::CoinAveragePrice { id, .. }
                | WorkerChannels::CoinAveragePriceCandles { id, .. } => id.clone(),
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { id, .. }
                | MarketChannels::CoinExchangeVolume { id, .. } => id.clone(),
            },
        }
    }

    pub fn get_coins(&self) -> &[String] {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::CoinAveragePrice { coins, .. }
                | WorkerChannels::CoinAveragePriceCandles { coins, .. } => coins,
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { coins, .. }
                | MarketChannels::CoinExchangeVolume { coins, .. } => coins,
            },
        }
    }

    pub fn get_frequency_ms(&self) -> u64 {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::CoinAveragePrice { frequency_ms, .. }
                | WorkerChannels::CoinAveragePriceCandles { frequency_ms, .. } => *frequency_ms,
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { frequency_ms, .. }
                | MarketChannels::CoinExchangeVolume { frequency_ms, .. } => *frequency_ms,
            },
        }
    }

    pub fn set_frequency_ms(&mut self, new_value: u64) {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::CoinAveragePrice { frequency_ms, .. }
                | WorkerChannels::CoinAveragePriceCandles { frequency_ms, .. } => {
                    *frequency_ms = new_value
                }
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { frequency_ms, .. }
                | MarketChannels::CoinExchangeVolume { frequency_ms, .. } => {
                    *frequency_ms = new_value
                }
            },
        }
    }

    pub fn get_method(&self) -> WsChannelName {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::CoinAveragePrice { .. } => WsChannelName::CoinAveragePrice,
                WorkerChannels::CoinAveragePriceCandles { .. } => {
                    WsChannelName::CoinAveragePriceCandles
                }
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { .. } => WsChannelName::CoinExchangePrice,
                MarketChannels::CoinExchangeVolume { .. } => WsChannelName::CoinExchangeVolume,
            },
        }
    }
}
