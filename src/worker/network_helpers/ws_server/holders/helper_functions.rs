use crate::config_scheme::async_from::AsyncFrom;
use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::{RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple};
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type HolderKey = (MarketValueOwner, MarketValue, Option<(String, String)>);
pub type HolderHashMap<T> = HashMap<HolderKey, Arc<RwLock<T>>>;

pub async fn make_holder_hashmap<
    T: AsyncFrom<(ConfigScheme, Option<RepositoryForF64ByTimestamp>)>,
>(
    config: &ConfigScheme,
    mut pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
) -> HolderHashMap<T> {
    let mut holder = HashMap::new();

    let key = (MarketValueOwner::Worker, MarketValue::IndexPrice, None);
    holder.insert(
        key,
        Arc::new(RwLock::new(T::from((config.clone(), None)).await)),
    );

    let mut futures = Vec::new();
    for exchange_pair in &config.market.exchange_pairs {
        let key = (
            MarketValueOwner::Worker,
            MarketValue::PairAveragePrice,
            Some(exchange_pair.clone()),
        );
        let repository = pair_average_price_repositories
            .as_mut()
            .and_then(|v| v.remove(exchange_pair));

        let future = async move { (key, T::from((config.clone(), repository)).await) };
        futures.push(future);
    }
    for (key, item) in futures::future::join_all(futures).await {
        holder.insert(key, Arc::new(RwLock::new(item)));
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
                holder.insert(
                    key,
                    Arc::new(RwLock::new(T::from((config.clone(), None)).await)),
                );
            }
        }
    }

    holder
}
