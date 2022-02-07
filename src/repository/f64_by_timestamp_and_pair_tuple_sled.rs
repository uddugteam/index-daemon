use crate::repository::repository::Repository;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::HashSet;
use std::str;
use std::sync::{Arc, Mutex};

pub type TimestampAndPairTuple = (DateTime<Utc>, (String, String));

pub struct F64ByTimestampAndPairTupleSled {
    entity_name: String,
    repository: Arc<Mutex<vsdbsled::Db>>,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampAndPairTupleSled {
    pub fn new(
        entity_name: String,
        repository: Arc<Mutex<vsdbsled::Db>>,
        frequency_ms: u64,
    ) -> Self {
        Self {
            entity_name,
            repository,
            frequency_ms,
            last_insert_timestamp: MIN_DATETIME,
        }
    }

    fn make_key(&self, primary: TimestampAndPairTuple) -> String {
        let timestamp = primary.0;
        let pair = primary.1;
        let pair = format!("{}_{}", pair.0, pair.1);

        format!(
            "{}__{}__{}",
            self.entity_name,
            pair,
            timestamp.timestamp_millis(),
        )
    }

    fn get_keys(&self) -> HashSet<String> {
        self.repository
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

    fn set_keys(&mut self, keys: HashSet<String>) {
        let mut keys_string = String::new();
        keys.into_iter().for_each(|v| keys_string += &(v + ","));
        let keys_string = keys_string.trim_end_matches(',');

        let _ = self.repository.lock().unwrap().insert("keys", keys_string);
    }

    fn add_key(&mut self, key: String) {
        let mut keys = self.get_keys();

        keys.insert(key);

        self.set_keys(keys);
    }

    fn remove_key(&mut self, key: &str) {
        let mut keys = self.get_keys();

        keys.remove(key);

        self.set_keys(keys);
    }
}

impl Repository<TimestampAndPairTuple, f64> for F64ByTimestampAndPairTupleSled {
    fn read(&self, primary: TimestampAndPairTuple) -> Result<Option<f64>, String> {
        let key = self.make_key(primary);

        self.repository
            .lock()
            .unwrap()
            .get(key)
            .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
            .map_err(|e| e.to_string())
    }

    fn insert(
        &mut self,
        primary: TimestampAndPairTuple,
        new_value: f64,
    ) -> Option<Result<(), String>> {
        let timestamp = primary.0;

        if (timestamp - self.last_insert_timestamp).num_milliseconds() as u64 > self.frequency_ms {
            // Enough time passed
            self.last_insert_timestamp = timestamp;

            let key = self.make_key(primary);

            self.add_key(key.clone());

            let res = self
                .repository
                .lock()
                .unwrap()
                .insert(key, new_value.to_ne_bytes())
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = self.repository.lock().unwrap().flush();

            Some(res)
        } else {
            // Too early
            None
        }
    }

    fn delete(&mut self, primary: TimestampAndPairTuple) {
        let key = self.make_key(primary);

        self.remove_key(&key);
        let _ = self.repository.lock().unwrap().remove(key);
        let _ = self.repository.lock().unwrap().flush();
    }
}
