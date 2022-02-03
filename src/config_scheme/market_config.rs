use crate::config_scheme::helper_functions::{
    get_config, get_default_channels, get_default_exchange_pairs, get_default_markets,
    get_param_value_as_vec_of_string, make_exchange_pairs,
};
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::market_channels::MarketChannels;

pub struct MarketConfig {
    pub markets: Vec<String>,
    pub exchange_pairs: Vec<ExchangePair>,
    pub channels: Vec<MarketChannels>,
}
impl MarketConfig {
    pub fn new() -> Self {
        let default = Self::default();
        let market_config = get_config("market_config");

        let markets = get_param_value_as_vec_of_string(&market_config, "exchanges")
            .unwrap_or(default.markets);
        let exchange_pairs = get_param_value_as_vec_of_string(&market_config, "coins")
            .map(|coins| make_exchange_pairs(coins, None))
            .unwrap_or(default.exchange_pairs);

        let channels = get_param_value_as_vec_of_string(&market_config, "channels");
        let channels = channels
            .map(|v| v.into_iter().map(|v| v.parse().unwrap()).collect())
            .unwrap_or(default.channels);

        Self {
            markets,
            exchange_pairs,
            channels,
        }
    }
}
impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            markets: get_default_markets(),
            exchange_pairs: get_default_exchange_pairs(),
            channels: get_default_channels(),
        }
    }
}
