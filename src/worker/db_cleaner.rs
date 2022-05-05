use crate::repository::repositories::{
    MarketRepositoriesByMarketName, RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple,
};
use crate::worker::helper_functions::date_time_subtract_sec;
use chrono::{DateTime, Utc};
use futures::FutureExt;

/// Seconds in minute
const INTERVAL_MINUTE: u64 = 60;

fn pick_with_interval(
    values: Vec<DateTime<Utc>>,
    interval_sec: u64,
) -> (Vec<DateTime<Utc>>, Vec<DateTime<Utc>>) {
    let mut keep = Vec::new();
    let mut discard = Vec::new();

    let mut next_timestamp = 0;
    for value in values {
        let curr_timestamp = value.timestamp() as u64;

        if curr_timestamp >= next_timestamp {
            keep.push(value);
            next_timestamp = curr_timestamp + interval_sec;
        } else {
            discard.push(value);
        }
    }

    (keep, discard)
}

async fn clear_repository(
    mut repository: RepositoryForF64ByTimestamp,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    log_identifier: String,
) {
    match repository.read_range(from..to).await {
        Ok(res) => {
            let keys = res.into_iter().map(|(k, _)| k).collect();
            let (_keep, discard) = pick_with_interval(keys, INTERVAL_MINUTE);

            repository.delete_multiple(&discard).await;
        }
        Err(e) => error!("{}. Read range error: {}", log_identifier, e),
    }
}

async fn clear_index_price_repository(
    repository: RepositoryForF64ByTimestamp,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) {
    clear_repository(
        repository,
        from,
        to,
        "clear_index_price_repository".to_string(),
    )
    .await;
}

async fn clear_pair_average_price_repositories(
    repositories: WorkerRepositoriesByPairTuple,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) {
    let fn_name = "clear_pair_average_price_repositories";
    let mut futures = Vec::new();

    for (pair_tuple, repository) in repositories {
        let log_identifier = format!("Fn: {}. Pair: {:?}", fn_name, pair_tuple);

        let future = clear_repository(repository, from, to, log_identifier);
        futures.push(future);
    }

    futures::future::join_all(futures).await;
}

async fn clear_market_repositories(
    repositories: MarketRepositoriesByMarketName,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) {
    let fn_name = "clear_market_repositories";
    let mut futures = Vec::new();

    for (market_name, repositories) in repositories {
        for (pair_tuple, repositories) in repositories {
            for (market_value, repository) in repositories {
                let market_name_2 = market_name.to_string();
                let pair_tuple_2 = pair_tuple.clone();

                let log_identifier = format!(
                    "Fn: {}. Market: {}. Pair: {:?}. Market value: {:?}",
                    fn_name, market_name_2, pair_tuple_2, market_value,
                );

                let future = clear_repository(repository, from, to, log_identifier);
                futures.push(future);
            }
        }
    }

    futures::future::join_all(futures).await;
}

pub async fn clear_db(
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    market_repositories: Option<MarketRepositoriesByMarketName>,
    data_expire_sec: u64,
) {
    info!("DB clearing begin.");

    let to = Utc::now();
    let from = date_time_subtract_sec(to, data_expire_sec);

    let mut futures = Vec::new();

    if let Some(index_price_repository) = index_price_repository {
        let future = clear_index_price_repository(index_price_repository, from, to);

        futures.push(future.boxed());
    }

    if let Some(pair_average_price_repositories) = pair_average_price_repositories {
        let future =
            clear_pair_average_price_repositories(pair_average_price_repositories, from, to);

        futures.push(future.boxed());
    }

    if let Some(market_repositories) = market_repositories {
        let future = clear_market_repositories(market_repositories, from, to);

        futures.push(future.boxed());
    }

    futures::future::join_all(futures).await;

    info!("DB clearing end.");
}
