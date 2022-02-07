use crate::repository::repository::Repository;
use chrono::{DateTime, Utc, MIN_DATETIME};
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

    fn delete(&self, primary: TimestampAndPairTuple) {
        let key = self.make_key(primary);

        let _ = self.repository.lock().unwrap().remove(key);
        let _ = self.repository.lock().unwrap().flush();
    }
}
