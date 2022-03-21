use crate::config_scheme::storage::Storage;
use crate::worker::defaults::{COINS, FIATS, MARKETS};
use crate::worker::market_helpers::market_channels::MarketChannels;
use clap::ArgMatches;
use env_logger::Builder;
use parse_duration::parse;

pub fn get_config_file_path(matches: &ArgMatches, key: &str) -> Option<String> {
    matches.value_of(key).map(|v| v.to_string())
}

pub fn get_config_from_config_files(matches: &ArgMatches, key: &str) -> config::Config {
    let mut config = config::Config::default();

    if let Some(path) = get_config_file_path(matches, key) {
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

pub fn get_default_exchange_pairs() -> Vec<(String, String)> {
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

pub fn get_default_historical() -> bool {
    false
}

pub fn get_default_storage(historical: bool) -> Option<Storage> {
    if historical {
        Some(Storage::default())
    } else {
        None
    }
}

pub fn get_default_data_expire_string() -> String {
    "1 month".to_string()
}

pub fn get_default_data_expire_sec() -> u64 {
    parse(&get_default_data_expire_string()).unwrap().as_secs()
}

pub fn make_pairs(coins: Vec<String>, fiats: Option<Vec<&str>>) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    let fiats = fiats.unwrap_or(FIATS.to_vec());

    for coin in coins {
        for fiat in &fiats {
            pairs.push((coin.to_string(), fiat.to_string()));
        }
    }

    pairs
}

pub fn make_exchange_pairs(coins: Vec<String>, fiats: Option<Vec<&str>>) -> Vec<(String, String)> {
    let mut exchange_pairs = Vec::new();

    let fiats = fiats.unwrap_or(FIATS.to_vec());

    for coin in coins {
        for fiat in &fiats {
            exchange_pairs.push((coin.to_string(), fiat.to_string()));
        }
    }

    exchange_pairs
}

pub fn is_subset<T: PartialEq>(set: &[T], subset: &[T]) -> bool {
    for item in subset {
        if !set.contains(item) {
            return false;
        }
    }

    true
}

pub fn has_no_duplicates<T: PartialEq>(set: &[T]) -> bool {
    let mut new_set = Vec::new();

    for item in set {
        if !new_set.contains(&item) {
            new_set.push(item);
        } else {
            return false;
        }
    }

    true
}
