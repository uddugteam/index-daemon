use crate::repository::repositories::{
    MarketRepositoriesByMarketName, RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple,
};
use crate::worker::helper_functions::date_time_subtract_sec;
use crate::worker::network_helpers::ws_server::interval::Interval;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::thread;

fn pick_with_interval(
    values: Vec<DateTime<Utc>>,
    interval: Interval,
) -> (Vec<DateTime<Utc>>, Vec<DateTime<Utc>>) {
    let interval = interval.into_seconds() as i64;

    let mut keep = Vec::new();
    let mut discard = Vec::new();

    let mut next_timestamp = MIN_DATETIME.timestamp();
    for value in values {
        let curr_timestamp = value.timestamp();

        if curr_timestamp >= next_timestamp {
            keep.push(value);
            next_timestamp = curr_timestamp + interval;
        } else {
            discard.push(value);
        }
    }

    (keep, discard)
}

fn clear_repository(
    mut repository: RepositoryForF64ByTimestamp,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    log_identifier: String,
) {
    match repository.read_range(from, to) {
        Ok(res) => {
            let keys = res.into_iter().map(|(k, _)| k).collect();
            let (_keep, discard) = pick_with_interval(keys, Interval::Minute);

            repository.delete_multiple(&discard);
        }
        Err(e) => error!("{}. Read range error: {}", log_identifier, e),
    }
}

fn clear_index_price_repository(
    repository: RepositoryForF64ByTimestamp,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) {
    clear_repository(
        repository,
        from,
        to,
        "clear_index_price_repository".to_string(),
    );
}

fn clear_pair_average_price_repositories(
    repositories: WorkerRepositoriesByPairTuple,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) {
    let fn_name = "clear_pair_average_price_repositories";
    let mut threads = Vec::new();

    for (pair_tuple, repository) in repositories {
        let thread_name = format!("fn: {}, pair: {:?}", fn_name, pair_tuple);

        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                clear_repository(
                    repository,
                    from,
                    to,
                    format!("Fn: {}. Pair: {:?}", fn_name, pair_tuple),
                );
            })
            .unwrap();

        threads.push(thread);
    }

    for thread in threads {
        let _ = thread.join();
    }
}

fn clear_market_repositories(
    repositories: MarketRepositoriesByMarketName,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) {
    let fn_name = "clear_market_repositories";
    let mut threads = Vec::new();

    for (market_name, repositories) in repositories {
        for (pair_tuple, repositories) in repositories {
            for (market_value, repository) in repositories {
                let market_name_2 = market_name.to_string();
                let pair_tuple_2 = pair_tuple.clone();

                let thread_name = format!(
                    "fn: {}, market: {}, pair: {:?}, market_value: {:?}",
                    fn_name, market_name, pair_tuple, market_value,
                );

                let thread = thread::Builder::new()
                    .name(thread_name)
                    .spawn(move || {
                        clear_repository(
                            repository,
                            from,
                            to,
                            format!(
                                "Fn: {}. Market: {}. Pair: {:?}. Market value: {:?}",
                                fn_name, market_name_2, pair_tuple_2, market_value,
                            ),
                        );
                    })
                    .unwrap();

                threads.push(thread);
            }
        }
    }

    for thread in threads {
        let _ = thread.join();
    }
}

pub fn clear_db(
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    market_repositories: Option<MarketRepositoriesByMarketName>,
    data_expire_sec: u64,
) {
    info!("DB clearing begin.");

    let to = Utc::now();
    let from = date_time_subtract_sec(to, data_expire_sec);

    let mut threads = Vec::new();

    if let Some(index_price_repository) = index_price_repository {
        let thread_name = "fn: clear_index_price_repository".to_string();

        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                clear_index_price_repository(index_price_repository, from, to);
            })
            .unwrap();

        threads.push(thread);
    }

    if let Some(pair_average_price_repositories) = pair_average_price_repositories {
        let thread_name = "fn: clear_pair_average_price_repositories".to_string();

        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                clear_pair_average_price_repositories(pair_average_price_repositories, from, to);
            })
            .unwrap();

        threads.push(thread);
    }

    if let Some(market_repositories) = market_repositories {
        let thread_name = "fn: clear_market_repositories".to_string();

        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                clear_market_repositories(market_repositories, from, to);
            })
            .unwrap();

        threads.push(thread);
    }

    for thread in threads {
        let _ = thread.join();
    }

    info!("DB clearing end.");
}
