use crate::repository::repository::Repository;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

pub type TimestampAndPairTuple = (DateTime<Utc>, (String, String));

pub struct F64ByTimestampAndPairTupleSled {
    entity_name: String,
    repository: Arc<Mutex<vsdbsled::Db>>,
}

impl F64ByTimestampAndPairTupleSled {
    pub fn new(entity_name: String, repository: Arc<Mutex<vsdbsled::Db>>) -> Self {
        Self {
            entity_name,
            repository,
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

    fn insert(&self, primary: TimestampAndPairTuple, new_value: f64) -> Result<(), String> {
        let key = self.make_key(primary);

        let res = self
            .repository
            .lock()
            .unwrap()
            .insert(key, new_value.to_ne_bytes())
            .map(|_| ())
            .map_err(|e| e.to_string());
        let _ = self.repository.lock().unwrap().flush();

        res
    }

    fn delete(&self, primary: TimestampAndPairTuple) {
        let key = self.make_key(primary);

        let _ = self.repository.lock().unwrap().remove(key);
        let _ = self.repository.lock().unwrap().flush();
    }
}
