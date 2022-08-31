use crate::config_scheme::market_config::MarketConfig;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderKey;
use num_format::{Locale, ToFormattedString};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{System, SystemExt};
use tokio::sync::RwLock;

type RocksdbHashmap =
    HashMap<HolderKey, (Arc<RwLock<rocksdb::DB>>, Arc<RwLock<HashMap<String, f64>>>)>;

#[derive(Debug)]
enum RamCheckTime {
    Before,
    After,
}

#[derive(Clone)]
pub enum Storage {
    Cache(Arc<RwLock<HashMap<String, f64>>>),
    Sled(vsdbsled::Db),
    Rocksdb(RocksdbHashmap),
}

impl Storage {
    pub fn new(name: &str, market_config: &MarketConfig) -> Result<Self, String> {
        let mut sys = System::new_all();
        Self::check_ram(&mut sys, RamCheckTime::Before);

        let res = match name {
            "cache" => Ok(Self::Cache(Arc::new(RwLock::new(HashMap::new())))),
            "sled" => Ok(Self::Sled(vsdbsled::open("db").unwrap())),
            "rocksdb" => Ok(Self::Rocksdb(Self::make_rocksdb_hashmap(market_config))),
            other_storage => Err(format!("Got wrong storage name: {}", other_storage)),
        };

        Self::check_ram(&mut sys, RamCheckTime::After);

        res
    }

    fn make_rocksdb_hashmap(market_config: &MarketConfig) -> RocksdbHashmap {
        let mut hash_map = HashMap::new();

        // ***************************************************************************************************************************
        // PairAveragePrice
        for exchange_pair in &market_config.exchange_pairs {
            let key = (
                MarketValueOwner::Worker,
                MarketValue::PairAveragePrice,
                Some(exchange_pair.clone()),
            );
            let pair = format!("{}_{}", exchange_pair.0, exchange_pair.1);
            let entity_name = format!(
                "worker__{}__{}",
                MarketValue::PairAveragePrice.to_string(),
                pair
            );

            hash_map.insert(
                key,
                (
                    Arc::new(RwLock::new(
                        rocksdb::DB::open_default(format!("db/{}", entity_name)).unwrap(),
                    )),
                    Arc::new(RwLock::new(HashMap::new())),
                ),
            );
        }
        // ###########################################################################################################################

        // ***************************************************************************************************************************
        // PairExchangePrice
        let market_values = [
            MarketValue::PairExchangePrice,
            MarketValue::PairExchangeVolume,
        ];
        for market_name in &market_config.markets {
            for exchange_pair in &market_config.exchange_pairs {
                for market_value in market_values {
                    let key = (
                        MarketValueOwner::Market(market_name.to_string()),
                        market_value,
                        Some(exchange_pair.clone()),
                    );
                    let pair = format!("{}_{}", exchange_pair.0, exchange_pair.1);
                    let entity_name = format!(
                        "market__{}__{}__{}",
                        market_name,
                        market_value.to_string(),
                        pair
                    );

                    hash_map.insert(
                        key,
                        (
                            Arc::new(RwLock::new(
                                rocksdb::DB::open_default(format!("db/{}", entity_name)).unwrap(),
                            )),
                            Arc::new(RwLock::new(HashMap::new())),
                        ),
                    );
                }
            }
        }
        // ###########################################################################################################################

        // ***************************************************************************************************************************
        // IndexPrice
        let entity_name = format!("worker__{}", MarketValue::IndexPrice.to_string());
        let key = (MarketValueOwner::Worker, MarketValue::IndexPrice, None);
        hash_map.insert(
            key,
            (
                Arc::new(RwLock::new(
                    rocksdb::DB::open_default(format!("db/{}", entity_name)).unwrap(),
                )),
                Arc::new(RwLock::new(HashMap::new())),
            ),
        );
        // ###########################################################################################################################

        hash_map
    }

    fn check_ram(sys: &mut System, time: RamCheckTime) {
        sys.refresh_all();
        let used_memory = (sys.used_memory() / 1024).to_formatted_string(&Locale::en);
        let total_memory = (sys.total_memory() / 1024).to_formatted_string(&Locale::en);
        debug!(
            "RAM {:?} DB open: {}/{} MB",
            time, used_memory, total_memory,
        );
    }
}
