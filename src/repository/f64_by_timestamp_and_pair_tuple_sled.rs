use crate::repository::repository::Repository;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

pub type TimestampAndPairTuple = (DateTime<Utc>, (String, String));

pub struct F64ByTimestampAndPairTupleSled(Arc<Mutex<vsdbsled::Db>>);

impl F64ByTimestampAndPairTupleSled {
    pub fn new(db: Arc<Mutex<vsdbsled::Db>>) -> Self {
        Self(db)
    }

    fn make_key(primary: TimestampAndPairTuple) -> String {
        let timestamp = primary.0;
        let pair = primary.1;

        format!(
            "{}__worker__pair_average_price__{}_{}",
            timestamp.timestamp_millis(),
            pair.0,
            pair.1,
        )
    }
}

impl Repository<TimestampAndPairTuple, f64> for F64ByTimestampAndPairTupleSled {
    fn read(&self, primary: TimestampAndPairTuple) -> Result<Option<f64>, String> {
        let key = Self::make_key(primary);

        self.0
            .lock()
            .unwrap()
            .get(key)
            .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
            .map_err(|e| e.to_string())
    }

    fn insert(&self, primary: TimestampAndPairTuple, new_value: f64) -> Result<(), String> {
        let key = Self::make_key(primary);

        let res = self
            .0
            .lock()
            .unwrap()
            .insert(key, new_value.to_ne_bytes())
            .map(|_| ())
            .map_err(|e| e.to_string());
        let _ = self.0.lock().unwrap().flush();

        res
    }

    fn delete(&self, primary: TimestampAndPairTuple) {
        let key = Self::make_key(primary);

        let _ = self.0.lock().unwrap().remove(key);
        let _ = self.0.lock().unwrap().flush();
    }
}
