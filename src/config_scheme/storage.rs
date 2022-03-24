use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub enum Storage {
    Sled(Arc<RwLock<vsdbsled::Db>>),
}

impl Storage {
    fn make_sled() -> Self {
        let tree = Arc::new(RwLock::new(vsdbsled::open("db").expect("Open db error.")));

        Self::Sled(tree)
    }

    pub fn from_str(name: &str) -> Self {
        match name {
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
