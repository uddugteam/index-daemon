use crate::config_scheme::helper_functions::{
    get_config, get_default_channels, get_default_coins, get_default_markets,
    get_param_value_as_vec_of_string,
};
use crate::worker::market_helpers::market_channels::MarketChannels;

pub struct MarketConfig {
    pub markets: Vec<String>,
    pub coins: Vec<String>,
    pub channels: Vec<MarketChannels>,
}
impl MarketConfig {
    pub fn new() -> Self {
        let default = Self::default();
        let market_config = get_config("market_config");

        let markets = get_param_value_as_vec_of_string(&market_config, "exchanges")
            .unwrap_or(default.markets);
        let coins =
            get_param_value_as_vec_of_string(&market_config, "coins").unwrap_or(default.coins);

        let channels = get_param_value_as_vec_of_string(&market_config, "channels");
        let channels = channels
            .map(|v| v.into_iter().map(|v| v.parse().unwrap()).collect())
            .unwrap_or(default.channels);

        Self {
            markets,
            coins,
            channels,
        }
    }
}
impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            markets: get_default_markets(),
            coins: get_default_coins(),
            channels: get_default_channels(),
        }
    }
}
