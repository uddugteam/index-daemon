use crate::config_scheme::async_from::AsyncFrom;
use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::{
    MarketRepositoriesByMarketName, RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple,
};
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type HolderKey = (MarketValueOwner, MarketValue, Option<(String, String)>);
pub type HolderHashMap<T> = HashMap<HolderKey, Arc<RwLock<T>>>;

async fn make_t<T: AsyncFrom<Option<RepositoryForF64ByTimestamp>> + Send + Sync + 'static>(
    key: HolderKey,
    repository: Option<RepositoryForF64ByTimestamp>,
) -> (HolderKey, T) {
    let debug = repository.is_some();
    if debug {
        debug!("fn make_holder_hashmap BEGIN: key: {:?}", key);
    }

    let res = (key.clone(), T::from(repository).await);

    if debug {
        debug!("fn make_holder_hashmap END: key: {:?}", key);
    }

    res
}

pub async fn make_holder_hashmap<
    T: AsyncFrom<Option<RepositoryForF64ByTimestamp>> + Send + Sync + 'static,
>(
    config: &ConfigScheme,
    mut pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    mut market_repositories: Option<MarketRepositoriesByMarketName>,
) -> HolderHashMap<T> {
    let mut holder = HashMap::new();
    let mut futures = Vec::new();

    let key = (MarketValueOwner::Worker, MarketValue::IndexPrice, None);
    holder.insert(key, Arc::new(RwLock::new(T::from(None).await)));

    for exchange_pair in &config.market.exchange_pairs {
        let key = (
            MarketValueOwner::Worker,
            MarketValue::PairAveragePrice,
            Some(exchange_pair.clone()),
        );
        let repository = pair_average_price_repositories
            .as_mut()
            .and_then(|v| v.remove(exchange_pair));

        let future = tokio::spawn(make_t(key, repository));
        futures.push(future.boxed());
    }

    let market_values = [
        MarketValue::PairExchangePrice,
        MarketValue::PairExchangeVolume,
    ];
    for market_name in &config.market.markets {
        for market_value in market_values {
            for exchange_pair in &config.market.exchange_pairs {
                let key = (
                    MarketValueOwner::Market(market_name.to_string()),
                    market_value,
                    Some(exchange_pair.clone()),
                );
                let repository = market_repositories.as_mut().and_then(|v| {
                    v.get_mut(market_name).and_then(|v| {
                        v.get_mut(exchange_pair)
                            .and_then(|v| v.remove(&market_value))
                    })
                });

                let future = tokio::spawn(make_t(key, repository));
                futures.push(future.boxed());
            }
        }
    }

    let future_results = futures::future::join_all(futures).await;
    let future_results = future_results.into_iter().map(Result::unwrap);
    for (key, item) in future_results {
        holder.insert(key, Arc::new(RwLock::new(item)));
    }

    holder
}
