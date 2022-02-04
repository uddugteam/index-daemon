use crate::config_scheme::market_config::MarketConfig;
use crate::repository::f64_by_timestamp_and_pair_tuple_sled::{
    F64ByTimestampAndPairTupleSled, TimestampAndPairTuple,
};
use crate::repository::repository::Repository;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type RepositoryForF64ByTimestampAndPairTuple =
    Box<dyn Repository<TimestampAndPairTuple, f64> + Send>;
pub type RepositoriesByPairTuple =
    HashMap<(String, String), RepositoryForF64ByTimestampAndPairTuple>;
pub type RepositoriesByMarketName = HashMap<String, RepositoriesByPairTuple>;

pub struct Repositories {
    pub pair_average_price: RepositoryForF64ByTimestampAndPairTuple,
    pub market_repositories: RepositoriesByMarketName,
}

impl Repositories {
    pub fn new(market_config: &MarketConfig) -> Self {
        let tree = Arc::new(Mutex::new(vsdbsled::open("db").expect("Open db error.")));

        let pair_average_price = Box::new(F64ByTimestampAndPairTupleSled::new(
            "worker__pair_average_price".to_string(),
            Arc::clone(&tree),
        ));

        let market_value = "pair_price";
        let mut market_repositories = HashMap::new();
        for market_name in &market_config.markets {
            let hash_map = market_repositories
                .entry(market_name.clone())
                .or_insert(HashMap::new());

            for exchange_pair in &market_config.exchange_pairs {
                let entity_name = format!("market__{}__{}", market_name, market_value);

                let repository: RepositoryForF64ByTimestampAndPairTuple = Box::new(
                    F64ByTimestampAndPairTupleSled::new(entity_name, Arc::clone(&tree)),
                );

                hash_map.insert(exchange_pair.pair.clone(), repository);
            }
        }

        Self {
            pair_average_price,
            market_repositories,
        }
    }
}
