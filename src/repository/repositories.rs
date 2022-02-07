use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::f64_by_timestamp_and_pair_tuple_sled::{
    F64ByTimestampAndPairTupleSled, TimestampAndPairTuple,
};
use crate::repository::repository::Repository;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type RepositoryForF64ByTimestampAndPairTuple =
    Box<dyn Repository<TimestampAndPairTuple, f64> + Send>;
pub type RepositoriesByMarketValue = HashMap<String, RepositoryForF64ByTimestampAndPairTuple>;
pub type RepositoriesByPairTuple = HashMap<(String, String), RepositoriesByMarketValue>;
pub type RepositoriesByMarketName = HashMap<String, RepositoriesByPairTuple>;

pub struct Repositories {
    pub pair_average_price: RepositoryForF64ByTimestampAndPairTuple,
    pub market_repositories: RepositoriesByMarketName,
}

impl Repositories {
    pub fn new(config: &ConfigScheme) -> Option<Self> {
        let market_config = &config.market;
        let service_config = &config.service;

        if service_config.historical {
            let (pair_average_price, market_repositories) = match service_config.storage.as_str() {
                "sled" => {
                    let tree = Arc::new(Mutex::new(vsdbsled::open("db").expect("Open db error.")));

                    let pair_average_price = Box::new(F64ByTimestampAndPairTupleSled::new(
                        "worker__pair_average_price".to_string(),
                        Arc::clone(&tree),
                        service_config.historical_storage_frequency_ms,
                    ));

                    let market_values = ["pair_price", "pair_volume"];
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
                                let entity_name =
                                    format!("market__{}__{}", market_name, market_value);

                                let repository: RepositoryForF64ByTimestampAndPairTuple =
                                    Box::new(F64ByTimestampAndPairTupleSled::new(
                                        entity_name,
                                        Arc::clone(&tree),
                                        service_config.historical_storage_frequency_ms,
                                    ));

                                hash_map.insert(market_value.to_string(), repository);
                            }
                        }
                    }

                    (pair_average_price, hash_map)
                }
                other_storage => panic!("Got wrong storage name: {}", other_storage),
            };

            Some(Self {
                pair_average_price,
                market_repositories,
            })
        } else {
            None
        }
    }

    pub fn optionize_fields(
        config: Option<Self>,
    ) -> (
        Option<RepositoryForF64ByTimestampAndPairTuple>,
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
}
