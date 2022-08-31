use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::{RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple};
use crate::worker::helper_functions::datetime_from_timestamp_sec;
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use futures::FutureExt;
use reqwest::Client;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

fn get_cli_param_values(matches: &ArgMatches, key: &str) -> Option<Vec<String>> {
    if matches.is_valid_arg(key) {
        matches
            .values_of(key)
            .map(|v| v.map(|v| v.to_string()).collect())
    } else {
        None
    }
}

fn parse_timestamp(
    fill_historical_config: &[String],
) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let timestamp = fill_historical_config[0].as_str();
    let timestamp: Vec<&str> = timestamp.split(',').collect();
    if timestamp.is_empty() {
        return Err("Got no timestamps.".to_string());
    }
    if timestamp.len() > 2 {
        return Err("Wrong timestamp format.".to_string());
    }

    if let Some(timestamp_from) = timestamp.first() {
        let timestamp_from = datetime_from_timestamp_sec(timestamp_from.parse().unwrap());

        let timestamp_to = timestamp
            .get(1)
            .map(|v| datetime_from_timestamp_sec(v.parse().unwrap()))
            .unwrap_or(Utc::now());

        if timestamp_from > timestamp_to {
            Err("\"timestamp_from\" must be lower or equal that \"timestamp_to\".".to_string())
        } else {
            Ok((timestamp_from, timestamp_to))
        }
    } else {
        Err("Wrong timestamp format.".to_string())
    }
}

fn parse_cryptocompare_json(json: serde_json::Value) -> Option<Vec<(DateTime<Utc>, f64)>> {
    let mut prices = Vec::new();

    let object = json.as_object()?;
    let array = object.get("Data")?.as_object()?.get("Data")?.as_array()?;
    for object in array {
        let object = object.as_object()?;

        let timestamp = object.get("time")?.as_u64()?;
        let timestamp = datetime_from_timestamp_sec(timestamp);

        let high_price = object.get("high")?.as_f64()?;
        let low_price = object.get("low")?.as_f64()?;
        let avg_price = (high_price + low_price) / 2.0;

        prices.push((timestamp, avg_price));
    }

    Some(prices)
}

async fn get_coin_daily_prices(
    coin: &str,
    timestamp_to: DateTime<Utc>,
    day_count: u64,
) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
    if day_count > 2000 {
        Err("\"day_count\" must be lower or equal that \"2000\".".to_string())
    } else {
        let second_coin = "USD";
        let timestamp_to = timestamp_to.timestamp();
        let api_key = "eb444b751a15aa9921cf7e14e4054ee42464eb152d86094fbfee9b8313fe895e";

        let uri = format!("https://min-api.cryptocompare.com/data/v2/histoday?fsym={}&tsym={}&toTs={}&limit={}&api_key={}", coin, second_coin,timestamp_to, day_count,api_key);

        let response = Client::new().get(uri).send().await;

        let response = response.map_err(|e| e.to_string())?;
        let response = response.text().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;

        parse_cryptocompare_json(json).ok_or("Error parsing json.".to_string())
    }
}

async fn get_all_daily_prices(
    coins: &[String],
    timestamp_to: DateTime<Utc>,
    day_count: u64,
) -> Result<HashMap<String, Vec<(DateTime<Utc>, f64)>>, String> {
    let mut all_prices = HashMap::new();

    // To prevent DDoS attack on cryptocompare.com
    sleep(Duration::from_secs(60)).await;

    for coin in coins {
        // To prevent DDoS attack on cryptocompare.com
        sleep(Duration::from_secs(10)).await;

        match get_coin_daily_prices(coin, timestamp_to, day_count).await {
            Ok(coin_prices) => {
                all_prices.insert(coin.to_string(), coin_prices);
            }
            Err(e) => {
                // Sleep 1 minute before index-daemon restart
                sleep(Duration::from_secs(60)).await;

                return Err(format!(
                    "Error getting response from cryptocompare.com: {}",
                    e
                ));
            }
        }
    }

    Ok(all_prices)
}

async fn fill_storage(
    mut repository: RepositoryForF64ByTimestamp,
    prices: &[(DateTime<Utc>, f64)],
) {
    for (timestamp, price) in prices.iter().cloned() {
        repository.insert(timestamp, price).await;
    }
}

async fn calculate_index_prices(
    all_prices: &HashMap<String, Vec<(DateTime<Utc>, f64)>>,
) -> Vec<(DateTime<Utc>, f64)> {
    let mut index_prices = Vec::new();

    let mut prices_by_timestamp = HashMap::new();

    for coin_prices in all_prices.values() {
        for (timestamp, price) in coin_prices {
            prices_by_timestamp
                .entry(*timestamp)
                .or_insert(Vec::new())
                .push(*price);
        }
    }

    for (timestamp, prices) in prices_by_timestamp {
        let count = prices.len();
        let sum: f64 = prices.into_iter().sum();
        let avg = sum / count as f64;

        index_prices.push((timestamp, avg));
    }

    index_prices
}

async fn fill_index_price_repository(
    all_prices: &HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    index_price_repository: RepositoryForF64ByTimestamp,
) {
    let index_prices = calculate_index_prices(all_prices).await;

    fill_storage(index_price_repository, &index_prices).await;
}

async fn fill_pair_average_price_repositories(
    all_prices: &HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    mut pair_average_price_repositories: WorkerRepositoriesByPairTuple,
) {
    let mut futures = Vec::new();

    for (coin, coin_prices) in all_prices {
        let pair_tuple = (coin.to_string(), "USD".to_string());

        let pair_average_price_repository =
            pair_average_price_repositories.remove(&pair_tuple).unwrap();

        let future = fill_storage(pair_average_price_repository, coin_prices);
        futures.push(future);
    }

    futures::future::join_all(futures).await;
}

pub async fn fill_historical_data(
    config: &ConfigScheme,
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
) {
    let matches = &config.matches;

    if let Some(fill_historical_config) = get_cli_param_values(matches, "fill_historical") {
        if config.service.storage.is_some() {
            info!("Fill historical data begin.");

            assert!(index_price_repository.is_some());
            assert!(pair_average_price_repositories.is_some());

            let (timestamp_from, timestamp_to) = parse_timestamp(&fill_historical_config).unwrap();
            let day_count = (timestamp_to - timestamp_from).num_days() as u64;

            let coins = fill_historical_config[1].as_str();
            let coins: Vec<String> = coins.split(',').map(|v| v.to_string()).collect();
            assert!(!coins.is_empty());

            let all_prices = get_all_daily_prices(&coins, timestamp_to, day_count)
                .await
                .unwrap();

            let future_1 =
                fill_index_price_repository(&all_prices, index_price_repository.unwrap());

            let future_2 = fill_pair_average_price_repositories(
                &all_prices,
                pair_average_price_repositories.unwrap(),
            );

            futures::future::join_all([future_1.boxed(), future_2.boxed()]).await;

            info!("Fill historical data end.");
        } else {
            panic!("Called with param \"fill_historical\", but with disabled DB.");
        }
    }
}
