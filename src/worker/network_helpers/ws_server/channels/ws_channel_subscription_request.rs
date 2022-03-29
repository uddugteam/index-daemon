use crate::worker::network_helpers::ws_server::channels::market_channels::MarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::WorkerChannels;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WsChannelSubscriptionRequest {
    WorkerChannels(WorkerChannels),
    MarketChannels(MarketChannels),
}

impl WsChannelSubscriptionRequest {
    pub fn get_id(&self) -> JsonRpcId {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::IndexPrice { id, .. }
                | WorkerChannels::IndexPriceCandles { id, .. }
                | WorkerChannels::CoinAveragePrice { id, .. }
                | WorkerChannels::CoinAveragePriceCandles { id, .. } => id.clone(),
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { id, .. }
                | MarketChannels::CoinExchangeVolume { id, .. } => id.clone(),
            },
        }
    }

    pub fn get_coins(&self) -> Option<&[String]> {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::CoinAveragePrice { coins, .. }
                | WorkerChannels::CoinAveragePriceCandles { coins, .. } => Some(coins),
                WorkerChannels::IndexPrice { .. } | WorkerChannels::IndexPriceCandles { .. } => {
                    None
                }
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { coins, .. }
                | MarketChannels::CoinExchangeVolume { coins, .. } => Some(coins),
            },
        }
    }

    pub fn get_frequency_ms(&self) -> Option<u64> {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::IndexPrice { frequency_ms, .. }
                | WorkerChannels::IndexPriceCandles { frequency_ms, .. }
                | WorkerChannels::CoinAveragePrice { frequency_ms, .. }
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
                WorkerChannels::IndexPrice { frequency_ms, .. }
                | WorkerChannels::IndexPriceCandles { frequency_ms, .. }
                | WorkerChannels::CoinAveragePrice { frequency_ms, .. }
                | WorkerChannels::CoinAveragePriceCandles { frequency_ms, .. } => {
                    *frequency_ms = Some(new_value)
                }
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice { frequency_ms, .. }
                | MarketChannels::CoinExchangeVolume { frequency_ms, .. } => {
                    *frequency_ms = Some(new_value)
                }
            },
        }
    }

    pub fn get_percent_change_interval_sec(&self) -> Option<u64> {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::IndexPrice {
                    percent_change_interval_sec,
                    ..
                }
                | WorkerChannels::CoinAveragePrice {
                    percent_change_interval_sec,
                    ..
                } => *percent_change_interval_sec,
                WorkerChannels::IndexPriceCandles { .. }
                | WorkerChannels::CoinAveragePriceCandles { .. } => None,
            },
            Self::MarketChannels(channel) => match channel {
                MarketChannels::CoinExchangePrice {
                    percent_change_interval_sec,
                    ..
                }
                | MarketChannels::CoinExchangeVolume {
                    percent_change_interval_sec,
                    ..
                } => *percent_change_interval_sec,
            },
        }
    }

    pub fn get_method(&self) -> WsChannelName {
        match self {
            Self::WorkerChannels(channel) => match channel {
                WorkerChannels::IndexPrice { .. } => WsChannelName::IndexPrice,
                WorkerChannels::IndexPriceCandles { .. } => WsChannelName::IndexPriceCandles,
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
