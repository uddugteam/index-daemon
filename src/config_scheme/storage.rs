use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{System, SystemExt};
use tokio::sync::RwLock;

#[derive(Debug)]
enum RamCheckTime {
    Before,
    After,
}

#[derive(Clone)]
pub enum Storage {
    Cache(Arc<RwLock<HashMap<String, f64>>>),
    Sled(vsdbsled::Db),
}

impl Storage {
    pub fn new(name: &str) -> Result<Self, String> {
        let mut sys = System::new_all();
        Self::check_ram(&mut sys, RamCheckTime::Before);

        let res = match name {
            "cache" => Ok(Self::Cache(Arc::new(RwLock::new(HashMap::new())))),
            "sled" => Ok(Self::make_sled()),
            other_storage => Err(format!("Got wrong storage name: {}", other_storage)),
        };

        Self::check_ram(&mut sys, RamCheckTime::After);

        res
    }

    fn make_sled() -> Self {
        let tree = vsdbsled::open("db").expect("Open db error.");

        Self::Sled(tree)
    }

    fn check_ram(sys: &mut System, time: RamCheckTime) {
        sys.refresh_all();
        let used_memory = sys.used_memory() / 1024;
        let total_memory = sys.total_memory() / 1024;
        debug!(
            "RAM {:?} DB open: {}/{} MB",
            time, used_memory, total_memory,
        );
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new("sled").unwrap()
    }
}
