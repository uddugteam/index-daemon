use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::storage::Storage;
use crate::repository::f64_by_timestamp_cache::F64ByTimestampCache;
use crate::repository::f64_by_timestamp_sled::F64ByTimestampSled;
use crate::repository::repository::Repository;
use crate::worker::market_helpers::market_value::MarketValue;
use chrono::{DateTime, Utc};
use redis::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type RepositoryForF64ByTimestamp = Box<dyn Repository<DateTime<Utc>, f64> + Send + Sync>;
pub type WorkerRepositoriesByPairTuple = HashMap<(String, String), RepositoryForF64ByTimestamp>;
pub type MarketRepositoriesByMarketValue = HashMap<MarketValue, RepositoryForF64ByTimestamp>;
pub type MarketRepositoriesByPairTuple = HashMap<(String, String), MarketRepositoriesByMarketValue>;
pub type MarketRepositoriesByMarketName = HashMap<String, MarketRepositoriesByPairTuple>;

pub struct Repositories {
    pub pair_average_price: WorkerRepositoriesByPairTuple,
    pub market_repositories: MarketRepositoriesByMarketName,
    pub index_repository: RepositoryForF64ByTimestamp,
}

impl Repositories {
    pub fn new(config: &ConfigScheme) -> Option<Self> {
        let service_config = &config.service;

        let cache = service_config.cache.as_ref().unwrap().clone();

        service_config.storage.as_ref().map(|storage| Self {
            pair_average_price: Self::make_pair_average_price(config, storage, &cache),
            market_repositories: Self::make_market_repositories(config, storage, &cache),
            index_repository: Self::make_index_repository(config, storage, &cache),
        })
    }

    pub fn optionize_fields(
        config: Option<Self>,
    ) -> (
        Option<WorkerRepositoriesByPairTuple>,
        Option<MarketRepositoriesByMarketName>,
        Option<RepositoryForF64ByTimestamp>,
    ) {
        match config {
            Some(config) => (
                Some(config.pair_average_price),
                Some(config.market_repositories),
                Some(config.index_repository),
            ),
            None => (None, None, None),
        }
    }

    fn make_repository(
        config: &ConfigScheme,
        storage: &Storage,
        cache: &Arc<Mutex<Connection>>,
        entity_name: String,
    ) -> RepositoryForF64ByTimestamp {
        match storage {
            Storage::Cache(arc) => Box::new(F64ByTimestampCache::new(
                entity_name,
                Arc::clone(arc),
                config.service.historical_storage_frequency_ms,
            )),
            Storage::Sled(tree) => Box::new(F64ByTimestampSled::new(
                entity_name,
                tree.clone(),
                Arc::clone(cache),
                config.service.historical_storage_frequency_ms,
            )),
        }
    }

    fn make_pair_average_price(
        config: &ConfigScheme,
        storage: &Storage,
        cache: &Arc<Mutex<Connection>>,
    ) -> WorkerRepositoriesByPairTuple {
        let market_config = &config.market;

        let mut hash_map = HashMap::new();
        for exchange_pair in &market_config.exchange_pairs {
            let pair = format!("{}_{}", exchange_pair.0, exchange_pair.1);
            let entity_name = format!(
                "worker__{}__{}",
                MarketValue::PairAveragePrice.to_string(),
                pair
            );

            let repository = Self::make_repository(config, storage, cache, entity_name);

            hash_map.insert(exchange_pair.clone(), repository);
        }

        hash_map
    }

    fn make_market_repositories(
        config: &ConfigScheme,
        storage: &Storage,
        cache: &Arc<Mutex<Connection>>,
    ) -> MarketRepositoriesByMarketName {
        let market_config = &config.market;

        let market_values = [
            MarketValue::PairExchangePrice,
            MarketValue::PairExchangeVolume,
        ];
        let mut hash_map = HashMap::new();
        for market_name in &market_config.markets {
            let hash_map = hash_map
                .entry(market_name.clone())
                .or_insert(HashMap::new());

            for exchange_pair in &market_config.exchange_pairs {
                let hash_map = hash_map
                    .entry(exchange_pair.clone())
                    .or_insert(HashMap::new());

                for market_value in market_values {
                    let pair = format!("{}_{}", exchange_pair.0, exchange_pair.1);
                    let entity_name = format!(
                        "market__{}__{}__{}",
                        market_name,
                        market_value.to_string(),
                        pair
                    );

                    let repository = Self::make_repository(config, storage, cache, entity_name);

                    hash_map.insert(market_value, repository);
                }
            }
        }

        hash_map
    }

    fn make_index_repository(
        config: &ConfigScheme,
        storage: &Storage,
        cache: &Arc<Mutex<Connection>>,
    ) -> RepositoryForF64ByTimestamp {
        let entity_name = format!("worker__{}", MarketValue::IndexPrice.to_string());

        Self::make_repository(config, storage, cache, entity_name)
    }
}
