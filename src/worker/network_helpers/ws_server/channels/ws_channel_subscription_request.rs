use crate::worker::network_helpers::ws_server::channels::market_channels::LocalMarketChannels;
use crate::worker::network_helpers::ws_server::channels::worker_channels::LocalWorkerChannels;
use crate::worker::network_helpers::ws_server::jsonrpc_request::JsonRpcId;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WsChannelSubscriptionRequest {
    Worker(LocalWorkerChannels),
    Market(LocalMarketChannels),
}

impl WsChannelSubscriptionRequest {
    pub fn get_id(&self) -> JsonRpcId {
        match self {
            Self::Worker(channel) => match channel {
                LocalWorkerChannels::IndexPrice { id, .. }
                | LocalWorkerChannels::IndexPriceCandles { id, .. }
                | LocalWorkerChannels::CoinAveragePrice { id, .. }
                | LocalWorkerChannels::CoinAveragePriceCandles { id, .. } => id.clone(),
            },
            Self::Market(channel) => match channel {
                LocalMarketChannels::CoinExchangePrice { id, .. }
                | LocalMarketChannels::CoinExchangeVolume { id, .. } => id.clone(),
            },
        }
    }

    pub fn get_coins(&self) -> Option<&[String]> {
        match self {
            Self::Worker(channel) => match channel {
                LocalWorkerChannels::CoinAveragePrice { coins, .. }
                | LocalWorkerChannels::CoinAveragePriceCandles { coins, .. } => Some(coins),
                LocalWorkerChannels::IndexPrice { .. }
                | LocalWorkerChannels::IndexPriceCandles { .. } => None,
            },
            Self::Market(channel) => match channel {
                LocalMarketChannels::CoinExchangePrice { coins, .. }
                | LocalMarketChannels::CoinExchangeVolume { coins, .. } => Some(coins),
            },
        }
    }

    pub fn get_exchanges(&self) -> Option<&[String]> {
        match self {
            Self::Worker(channel) => match channel {
                LocalWorkerChannels::CoinAveragePrice { .. }
                | LocalWorkerChannels::CoinAveragePriceCandles { .. }
                | LocalWorkerChannels::IndexPrice { .. }
                | LocalWorkerChannels::IndexPriceCandles { .. } => None,
            },
            Self::Market(channel) => match channel {
                LocalMarketChannels::CoinExchangePrice { exchanges, .. }
                | LocalMarketChannels::CoinExchangeVolume { exchanges, .. } => Some(exchanges),
            },
        }
    }

    pub fn get_frequency_ms(&self) -> u64 {
        match self {
            Self::Worker(channel) => match channel {
                LocalWorkerChannels::IndexPrice { frequency_ms, .. }
                | LocalWorkerChannels::IndexPriceCandles { frequency_ms, .. }
                | LocalWorkerChannels::CoinAveragePrice { frequency_ms, .. }
                | LocalWorkerChannels::CoinAveragePriceCandles { frequency_ms, .. } => {
                    *frequency_ms
                }
            },
            Self::Market(channel) => match channel {
                LocalMarketChannels::CoinExchangePrice { frequency_ms, .. }
                | LocalMarketChannels::CoinExchangeVolume { frequency_ms, .. } => *frequency_ms,
            },
        }
    }

    pub fn get_percent_change_interval_sec(&self) -> Option<u64> {
        match self {
            Self::Worker(channel) => match channel {
                LocalWorkerChannels::IndexPrice {
                    percent_change_interval_sec,
                    ..
                }
                | LocalWorkerChannels::CoinAveragePrice {
                    percent_change_interval_sec,
                    ..
                } => Some(*percent_change_interval_sec),
                LocalWorkerChannels::IndexPriceCandles { .. }
                | LocalWorkerChannels::CoinAveragePriceCandles { .. } => None,
            },
            Self::Market(channel) => match channel {
                LocalMarketChannels::CoinExchangePrice {
                    percent_change_interval_sec,
                    ..
                }
                | LocalMarketChannels::CoinExchangeVolume {
                    percent_change_interval_sec,
                    ..
                } => Some(*percent_change_interval_sec),
            },
        }
    }

    pub fn get_method(&self) -> WsChannelName {
        match self {
            Self::Worker(channel) => match channel {
                LocalWorkerChannels::IndexPrice { .. } => WsChannelName::IndexPrice,
                LocalWorkerChannels::IndexPriceCandles { .. } => WsChannelName::IndexPriceCandles,
                LocalWorkerChannels::CoinAveragePrice { .. } => WsChannelName::CoinAveragePrice,
                LocalWorkerChannels::CoinAveragePriceCandles { .. } => {
                    WsChannelName::CoinAveragePriceCandles
                }
            },
            Self::Market(channel) => match channel {
                LocalMarketChannels::CoinExchangePrice { .. } => WsChannelName::CoinExchangePrice,
                LocalMarketChannels::CoinExchangeVolume { .. } => WsChannelName::CoinExchangeVolume,
            },
        }
    }
}
