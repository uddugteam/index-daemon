use crate::config_scheme::market_config::MarketConfig;
use crate::repository::f64_by_timestamp_and_pair_tuple_sled::{
    F64ByTimestampAndPairTupleSled, TimestampAndPairTuple,
};
use crate::repository::repository::Repository;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Repositories {
    pub pair_average_price: Box<dyn Repository<TimestampAndPairTuple, f64> + Send>,
    pub market_repositories:
        HashMap<(String, String), Box<dyn Repository<TimestampAndPairTuple, f64> + Send>>,
}

impl Repositories {
    pub fn new(market_config: &MarketConfig) -> Self {
        let tree = Arc::new(Mutex::new(vsdbsled::open("db").expect("Open db error.")));

        let pair_average_price = Box::new(F64ByTimestampAndPairTupleSled::new(
            "worker__pair_average_price".to_string(),
            Arc::clone(&tree),
        ));

        let market_values = ["pair_price", "pair_volume"];
        let mut market_repositories = HashMap::new();
        for market_name in &market_config.markets {
            for market_value in market_values {
                let entity_name = format!("market__{}__{}", market_name, market_value);

                let repository: Box<dyn Repository<TimestampAndPairTuple, f64> + Send> = Box::new(
                    F64ByTimestampAndPairTupleSled::new(entity_name, Arc::clone(&tree)),
                );

                market_repositories.insert(
                    (market_name.to_string(), market_value.to_string()),
                    repository,
                );
            }
        }

        Self {
            pair_average_price,
            market_repositories,
        }
    }
}
