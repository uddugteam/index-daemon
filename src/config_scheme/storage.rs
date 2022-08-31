use crate::config_scheme::market_config::MarketConfig;
use crate::worker::network_helpers::ws_server::holders::helper_functions::{
    make_holder_keys, stringify_holderkey, HolderKey,
};
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

        for key in make_holder_keys(market_config) {
            let entity_name = stringify_holderkey(&key);

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
