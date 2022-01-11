use clap::{App, Arg};
use env_logger::Builder;
use std::sync::{mpsc, Arc, Mutex};

use crate::repository::pair_average_trade_price::PairAverageTradePrice;
use crate::worker::worker::Worker;

#[macro_use]
extern crate log;

mod repository;
mod worker;

fn get_config_file_path(key: &str) -> Option<String> {
    let matches = App::new("ICEX")
        .version("1.0")
        .arg(
            Arg::with_name("service_config")
                .long("service_config")
                .value_name("PATH")
                .help("Service config file path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("market_config")
                .long("market_config")
                .value_name("PATH")
                .help("Market config file path")
                .takes_value(true),
        )
        .get_matches();

    matches.value_of(key).map(|v| v.to_string())
}

fn get_config(key: &str) -> config::Config {
    let mut market_config = config::Config::default();

    if let Some(path) = get_config_file_path(key) {
        market_config.merge(config::File::with_name(&path)).unwrap();
    } else {
        let env_key = "APP__".to_string() + &key.to_uppercase() + "_";

        market_config
            .merge(config::Environment::with_prefix(&env_key).separator("__"))
            .unwrap();
    }

    market_config
}

fn get_param_value_as_vec_of_string(config: &config::Config, key: &str) -> Option<Vec<String>> {
    if let Ok(string) = config.get_str(key) {
        Some(string.split(',').map(|v| v.to_string()).collect())
    } else {
        config
            .get_array(key)
            .ok()
            .map(|v| v.into_iter().map(|v| v.into_str().unwrap()).collect())
    }
}

fn main() {
    let service_config = get_config("service_config");

    let log_level: String = service_config
        .get_str("log_level")
        .unwrap_or("trace".to_string());

    let mut builder = Builder::from_default_env();
    builder.filter(Some("index_daemon"), log_level.parse().unwrap());
    builder.init();

    let market_config = get_config("market_config");

    let markets: Option<Vec<String>> =
        get_param_value_as_vec_of_string(&market_config, "exchanges");
    let markets: Option<Vec<&str>> = markets
        .as_ref()
        .map(|v| v.iter().map(|v| v.as_str()).collect());

    let coins: Option<Vec<String>> = get_param_value_as_vec_of_string(&market_config, "coins");
    let coins: Option<Vec<&str>> = coins
        .as_ref()
        .map(|v| v.iter().map(|v| v.as_str()).collect());

    let channels: Option<Vec<String>> =
        get_param_value_as_vec_of_string(&market_config, "channels");
    let channels: Option<Vec<&str>> = channels
        .as_ref()
        .map(|v| v.iter().map(|v| v.as_str()).collect());

    let (tx, rx) = mpsc::channel();
    let pair_average_trade_price_repository = PairAverageTradePrice::new();
    let worker = Worker::new(
        tx,
        Arc::new(Mutex::new(pair_average_trade_price_repository)),
    );
    worker.lock().unwrap().start(markets, coins, channels);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
