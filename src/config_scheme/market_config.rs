use crate::config_scheme::helper_functions::{
    get_config_from_config_files, get_default_channels, get_default_exchange_pairs,
    get_default_markets, get_param_value_as_vec_of_string, has_no_duplicates, is_subset,
    make_exchange_pairs, make_pairs,
};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use clap::ArgMatches;

#[derive(Clone)]
pub struct MarketConfig {
    pub markets: Vec<String>,
    pub exchange_pairs: Vec<(String, String)>,
    pub index_pairs: Vec<(String, String)>,
    pub channels: Vec<ExternalMarketChannels>,
}

impl MarketConfig {
    pub fn new(matches: &ArgMatches) -> Self {
        let default = Self::default();
        let market_config = get_config_from_config_files(matches, "market_config");

        let markets = get_param_value_as_vec_of_string(&market_config, "exchanges")
            .unwrap_or(default.markets);

        let exchange_pairs = get_param_value_as_vec_of_string(&market_config, "coins")
            .map(|coins| make_exchange_pairs(coins, None))
            .unwrap_or(default.exchange_pairs);

        let index_pairs = get_param_value_as_vec_of_string(&market_config, "index_coins")
            .map(|coins| make_pairs(coins, None))
            .unwrap_or(default.index_pairs);

        assert!(is_subset(&exchange_pairs, &index_pairs));
        assert!(has_no_duplicates(&exchange_pairs));

        let channels = get_param_value_as_vec_of_string(&market_config, "channels");
        let channels = channels
            .map(|v| v.into_iter().map(|v| v.parse().unwrap()).collect())
            .unwrap_or(default.channels);

        Self {
            markets,
            exchange_pairs,
            index_pairs,
            channels,
        }
    }
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            markets: get_default_markets(),
            exchange_pairs: get_default_exchange_pairs(),
            index_pairs: get_default_exchange_pairs(),
            channels: get_default_channels(),
        }
    }
}
