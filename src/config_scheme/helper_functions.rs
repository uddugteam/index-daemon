use crate::config_scheme::storage::Storage;
use crate::worker::defaults::{COINS, FIATS, MARKETS};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use clap::ArgMatches;
use env_logger::Builder;
use parse_duration::parse;
use std::collections::HashSet;
use std::hash::Hash;

pub fn get_config_file_path(matches: &ArgMatches, key: &str) -> Option<String> {
    matches.value_of(key).map(|v| v.to_string())
}

pub fn get_config_from_config_files(matches: &ArgMatches, key: &str) -> config::Config {
    let config = config::Config::builder();

    let config = if let Some(path) = get_config_file_path(matches, key) {
        config.add_source(config::File::with_name(&path))
    } else {
        let env_key = "APP__".to_string() + &key.to_uppercase();

        config.add_source(config::Environment::with_prefix(&env_key).separator("__"))
    };

    config.build().unwrap()
}

pub fn get_param_value_as_vec_of_string(config: &config::Config, key: &str) -> Option<Vec<String>> {
    if let Ok(string) = config.get_string(key) {
        Some(string.split(',').map(|v| v.to_string()).collect())
    } else {
        config
            .get_array(key)
            .ok()
            .map(|v| v.into_iter().map(|v| v.into_string().unwrap()).collect())
    }
}

pub fn set_log_level(service_config: &config::Config) {
    let log_level = service_config
        .get_string("log_level")
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

pub fn get_default_channels() -> Vec<ExternalMarketChannels> {
    ExternalMarketChannels::get_all().to_vec()
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

pub fn get_default_percent_change_interval_string() -> String {
    "1 minute".to_string()
}
pub fn get_default_percent_change_interval_sec() -> u64 {
    parse(&get_default_percent_change_interval_string())
        .unwrap()
        .as_secs()
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

pub fn set_intersection<T: PartialEq + Eq + Hash + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let a: HashSet<&T> = a.iter().collect();
    let b: HashSet<&T> = b.iter().collect();

    a.intersection(&b).cloned().cloned().collect()
}

pub fn is_subset<T: PartialEq + Eq + Hash + Clone>(set: &[T], subset: &[T]) -> bool {
    let set: HashSet<&T> = set.iter().collect();
    let subset: HashSet<&T> = subset.iter().collect();

    // `subset` is a subset of `set`
    subset.is_subset(&set)
}

pub fn remove_duplicates<T: PartialEq + Eq + Hash>(input: Vec<T>, key: &str) -> Vec<T> {
    let input_len = input.len();
    let hash_set: HashSet<T> = input.into_iter().collect();

    if input_len != hash_set.len() {
        warn!(
            "Warning: You have duplicates in {}. Duplicates has been removed.",
            key
        );
    }

    hash_set.into_iter().collect()
}
