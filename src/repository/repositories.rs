use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::storage::Storage;
use crate::repository::f64_by_timestamp_cache::F64ByTimestampCache;
use crate::repository::f64_by_timestamp_rocksdb::F64ByTimestampRocksdb;
use crate::repository::f64_by_timestamp_sled::F64ByTimestampSled;
use crate::repository::repository::Repository;
use crate::worker::market_helpers::market_value::MarketValue;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;

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

        service_config.storage.as_ref().map(|storage| Self {
            pair_average_price: Self::make_pair_average_price(config, storage),
            market_repositories: Self::make_market_repositories(config, storage),
            index_repository: Self::make_index_repository(config, storage),
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
        entity_name: String,
        cache: bool,
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
                config.service.historical_storage_frequency_ms,
            )),
            Storage::Rocksdb(arc_repo, arc_cache) => Box::new(F64ByTimestampRocksdb::new(
                entity_name,
                Arc::clone(arc_repo),
                cache.then_some(Arc::clone(arc_cache)),
                config.service.historical_storage_frequency_ms,
            )),
        }
    }

    fn make_pair_average_price(
        config: &ConfigScheme,
        storage: &Storage,
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

            let repository = Self::make_repository(config, storage, entity_name, true);

            hash_map.insert(exchange_pair.clone(), repository);
        }

        hash_map
    }

    fn make_market_repositories(
        config: &ConfigScheme,
        storage: &Storage,
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

                    let repository = Self::make_repository(config, storage, entity_name, false);

                    hash_map.insert(market_value, repository);
                }
            }
        }

        hash_map
    }

    fn make_index_repository(
        config: &ConfigScheme,
        storage: &Storage,
    ) -> RepositoryForF64ByTimestamp {
        let entity_name = format!("worker__{}", MarketValue::IndexPrice.to_string());

        Self::make_repository(config, storage, entity_name, false)
    }
}
