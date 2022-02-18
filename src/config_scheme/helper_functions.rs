use crate::worker::defaults::{COINS, FIATS, MARKETS};
use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::market_channels::MarketChannels;
use clap::{App, Arg, ValueHint};
use env_logger::Builder;

pub fn get_config_file_path(key: &str) -> Option<String> {
    let matches = App::new("ICEX")
        .version("1.0")
        .arg(
            Arg::new("service_config")
                .long("service_config")
                .value_name("PATH")
                .help("Service config file path")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            Arg::new("market_config")
                .long("market_config")
                .value_name("PATH")
                .help("Market config file path")
                .value_hint(ValueHint::FilePath),
        )
        .get_matches();

    matches.value_of(key).map(|v| v.to_string())
}

pub fn get_config_from_config_files(key: &str) -> config::Config {
    let mut config = config::Config::default();

    if let Some(path) = get_config_file_path(key) {
        config.merge(config::File::with_name(&path)).unwrap();
    } else {
        let env_key = "APP__".to_string() + &key.to_uppercase() + "_";

        config
            .merge(config::Environment::with_prefix(&env_key).separator("__"))
            .unwrap();
    }

    config
}

pub fn get_param_value_as_vec_of_string(config: &config::Config, key: &str) -> Option<Vec<String>> {
    if let Ok(string) = config.get_str(key) {
        Some(string.split(',').map(|v| v.to_string()).collect())
    } else {
        config
            .get_array(key)
            .ok()
            .map(|v| v.into_iter().map(|v| v.into_str().unwrap()).collect())
    }
}

pub fn set_log_level(service_config: &config::Config) {
    let log_level = service_config
        .get_str("log_level")
        .unwrap_or("trace".to_string());

    let mut builder = Builder::from_default_env();
    builder.filter(Some("index_daemon"), log_level.parse().unwrap());
    builder.init();
}

pub fn get_default_markets() -> Vec<String> {
    MARKETS.into_iter().map(|v| v.to_string()).collect()
}

pub fn get_default_coins() -> Vec<String> {
    COINS.into_iter().map(|v| v.to_string()).collect()
}

pub fn get_default_exchange_pairs() -> Vec<ExchangePair> {
    make_exchange_pairs(get_default_coins(), None)
}

pub fn get_default_channels() -> Vec<MarketChannels> {
    MarketChannels::get_all().to_vec()
}

pub fn get_default_host() -> String {
    "127.0.0.1".to_string()
}

pub fn get_default_port() -> String {
    "8080".to_string()
}

pub fn make_exchange_pairs(coins: Vec<String>, fiats: Option<Vec<&str>>) -> Vec<ExchangePair> {
    let mut exchange_pairs = Vec::new();

    let fiats = fiats.unwrap_or(FIATS.to_vec());

    for coin in coins {
        for fiat in &fiats {
            exchange_pairs.push(ExchangePair {
                pair: (coin.to_string(), fiat.to_string()),
                conversion: ConversionType::None,
            });
        }
    }

    exchange_pairs
}
