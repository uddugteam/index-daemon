use crate::config_scheme::helper_functions::{get_config, get_param_value_as_vec_of_string};

pub struct MarketConfig {
    pub markets: Option<Vec<String>>,
    pub coins: Option<Vec<String>>,
    pub channels: Option<Vec<String>>,
}
impl MarketConfig {
    pub fn new() -> Self {
        let market_config = get_config("market_config");

        let markets = get_param_value_as_vec_of_string(&market_config, "exchanges");
        let coins = get_param_value_as_vec_of_string(&market_config, "coins");
        let channels = get_param_value_as_vec_of_string(&market_config, "channels");

        Self {
            markets,
            coins,
            channels,
        }
    }
}
