use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub enum Storage {
    Cache(Arc<RwLock<HashMap<String, f64>>>),
    Sled(vsdbsled::Db),
}

impl Storage {
    fn make_sled() -> Self {
        let tree = vsdbsled::open("db").expect("Open db error.");

        Self::Sled(tree)
    }

    pub fn from_str(name: &str) -> Self {
        match name {
            "cache" => Self::Cache(Arc::new(RwLock::new(HashMap::new()))),
            "sled" => Self::make_sled(),
            other_storage => panic!("Got wrong storage name: {}", other_storage),
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::make_sled()
    }
}
