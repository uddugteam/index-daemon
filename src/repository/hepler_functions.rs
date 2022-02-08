use std::collections::HashSet;
use std::str;
use std::sync::{Arc, Mutex};

pub fn get_all_keys_sled(repository: &Arc<Mutex<vsdbsled::Db>>) -> HashSet<String> {
    repository
        .lock()
        .unwrap()
        .get("keys")
        .map(|v| v.map(|v| str::from_utf8(&v.to_vec()).unwrap().to_string()))
        .unwrap()
        .unwrap()
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect()
}
