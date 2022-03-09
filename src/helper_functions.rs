use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::storage::Storage;
use crate::repository::repositories::{
    Repositories, RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple,
};
use crate::worker::helper_functions::date_time_from_timestamp_sec;
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use reqwest::blocking::Client;
use std::sync::Arc;
use std::thread;
use std::time;

fn get_cli_param_values(matches: &ArgMatches, key: &str) -> Option<Vec<String>> {
    matches
        .values_of(key)
        .map(|v| v.map(|v| v.to_string()).collect())
}

fn parse_timestamp(fill_historical_config: &[String]) -> (DateTime<Utc>, DateTime<Utc>) {
    let timestamp = fill_historical_config[0].as_str();
    let timestamp: Vec<&str> = timestamp.split(',').collect();
    assert!(timestamp.len() <= 2);

    let (timestamp_from, timestamp_to) = if let Some(timestamp_from) = timestamp.get(0) {
        let timestamp_from = date_time_from_timestamp_sec(timestamp_from.parse().unwrap());

        let timestamp_to = timestamp
            .get(1)
            .map(|v| date_time_from_timestamp_sec(v.parse().unwrap()))
            .unwrap_or(Utc::now());

        (timestamp_from, timestamp_to)
    } else {
        panic!("Wrong timestamp format.");
    };
    assert!(timestamp_to > timestamp_from);

    (timestamp_from, timestamp_to)
}

fn get_daily_prices(
    coin: &str,
    timestamp_to: DateTime<Utc>,
    day_count: u64,
) -> Option<Vec<(DateTime<Utc>, f64)>> {
    assert!(day_count <= 2000);

    let second_coin = "USD";
    let timestamp_to = timestamp_to.timestamp();
    let api_key = "eb444b751a15aa9921cf7e14e4054ee42464eb152d86094fbfee9b8313fe895e";

    let uri = format!("https://min-api.cryptocompare.com/data/v2/histoday?fsym={}&tsym={}&toTs={}&limit={}&api_key={}", coin, second_coin,timestamp_to, day_count,api_key);

    let response = Client::new().get(uri).send();

    let response = response.ok()?;
    let response = response.text().ok()?;
    let json: serde_json::Value = serde_json::from_str(&response).ok()?;

    let mut prices = Vec::new();

    let object = json.as_object()?;
    let array = object.get("Data")?.as_object()?.get("Data")?.as_array()?;
    for object in array {
        let object = object.as_object()?;

        let timestamp = object.get("time")?.as_u64()?;
        let timestamp = date_time_from_timestamp_sec(timestamp);

        let high_price = object.get("high")?.as_f64()?;
        let low_price = object.get("low")?.as_f64()?;
        let avg_price = (high_price + low_price) / 2.0;

        prices.push((timestamp, avg_price));
    }

    Some(prices)
}

fn make_repositories(config: &ConfigScheme) -> WorkerRepositoriesByPairTuple {
    match config.service.storage.as_ref().unwrap() {
        Storage::Sled(tree) => Repositories::make_pair_average_price_sled(config, Arc::clone(tree)),
    }
}

fn fill_storage(
    mut repository: RepositoryForF64ByTimestamp,
    prices: Vec<(DateTime<Utc>, f64)>,
    coin: String,
) {
    info!("Fill historical data for {} begin.", coin);

    for (timestamp, price) in prices {
        repository.insert(timestamp, price);
    }

    info!("Fill historical data for {} end.", coin);
}

pub fn fill_historical_data(config: &ConfigScheme) {
    info!("Fill historical data begin.");

    if config.service.storage.is_some() {
        let matches = &config.matches;

        if let Some(fill_historical_config) = get_cli_param_values(matches, "fill_historical") {
            let (timestamp_from, timestamp_to) = parse_timestamp(&fill_historical_config);
            let day_count = (timestamp_to - timestamp_from).num_days() as u64;

            let coins = fill_historical_config[1].as_str();
            let coins: Vec<String> = coins.split(',').map(|v| v.to_string()).collect();
            assert!(!coins.is_empty());

            let mut repositories = make_repositories(config);

            let mut threads = Vec::new();
            for coin in coins {
                let pair_tuple = (coin.to_string(), "USD".to_string());
                let prices = get_daily_prices(&coin, timestamp_to, day_count).unwrap();
                let repository = repositories.remove(&pair_tuple).unwrap();

                let thread = thread::spawn(move || fill_storage(repository, prices, coin));
                threads.push(thread);

                // To prevent DDoS attack on cryptocompare.com
                thread::sleep(time::Duration::from_millis(10000));
            }
            for thread in threads {
                let _ = thread.join();
            }
        }
    }

    info!("Fill historical data end.");
}
