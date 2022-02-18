use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::storage::Storage;
use crate::repository::f64_by_timestamp_sled::F64ByTimestampSled;
use crate::repository::repository::Repository;
use crate::worker::market_helpers::market_value::MarketValue;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type RepositoryForF64ByTimestamp = Box<dyn Repository<DateTime<Utc>, f64> + Send>;
pub type RepositoriesByMarketValue = HashMap<MarketValue, RepositoryForF64ByTimestamp>;
pub type RepositoriesByPairTuple = HashMap<(String, String), RepositoriesByMarketValue>;
pub type RepositoriesByMarketName = HashMap<String, RepositoriesByPairTuple>;

pub struct Repositories {
    pub pair_average_price: RepositoryForF64ByTimestamp,
    pub market_repositories: RepositoriesByMarketName,
}

impl Repositories {
    pub fn new(config: &ConfigScheme) -> Option<Self> {
        let service_config = &config.service;

        match &service_config.storage {
            Some(storage) => match storage {
                Storage::Sled(tree) => Some(Self {
                    pair_average_price: Self::make_pair_average_price_sled(
                        config,
                        Arc::clone(tree),
                    ),
                    market_repositories: Self::make_market_repositories_sled(
                        config,
                        Arc::clone(tree),
                    ),
                }),
            },
            None => None,
        }
    }

    pub fn optionize_fields(
        config: Option<Self>,
    ) -> (
        Option<RepositoryForF64ByTimestamp>,
        Option<RepositoriesByMarketName>,
    ) {
        match config {
            Some(config) => (
                Some(config.pair_average_price),
                Some(config.market_repositories),
            ),
            None => (None, None),
        }
    }

    pub fn make_pair_average_price_sled(
        config: &ConfigScheme,
        tree: Arc<Mutex<vsdbsled::Db>>,
    ) -> RepositoryForF64ByTimestamp {
        Box::new(F64ByTimestampSled::new(
            "worker__".to_string() + &MarketValue::PairAveragePrice.to_string(),
            Arc::clone(&tree),
            config.service.historical_storage_frequency_ms,
        ))
    }

    fn make_market_repositories_sled(
        config: &ConfigScheme,
        tree: Arc<Mutex<vsdbsled::Db>>,
    ) -> RepositoriesByMarketName {
        let market_config = &config.market;
        let service_config = &config.service;

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
                    .entry(exchange_pair.pair.clone())
                    .or_insert(HashMap::new());

                for market_value in market_values {
                    let pair = format!("{}_{}", exchange_pair.pair.0, exchange_pair.pair.1);
                    let entity_name = format!(
                        "market__{}__{}__{}",
                        market_name,
                        market_value.to_string(),
                        pair
                    );

                    let repository: RepositoryForF64ByTimestamp =
                        Box::new(F64ByTimestampSled::new(
                            entity_name,
                            Arc::clone(&tree),
                            service_config.historical_storage_frequency_ms,
                        ));

                    hash_map.insert(market_value, repository);
                }
            }
        }

        hash_map
    }
}
