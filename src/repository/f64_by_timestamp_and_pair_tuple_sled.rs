use crate::repository::hepler_functions::get_all_keys_sled;
use crate::repository::repository::Repository;
use crate::worker::helper_functions::date_time_from_timestamp;
use chrono::{DateTime, Utc, MIN_DATETIME};
use std::collections::{HashMap, HashSet};
use std::str;
use std::sync::{Arc, Mutex};

pub type TimestampAndPairTuple = (DateTime<Utc>, (String, String));

#[derive(Clone)]
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

    fn stringify_primary(&self, primary: TimestampAndPairTuple) -> String {
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

    fn get_all_keys(&self) -> HashSet<String> {
        get_all_keys_sled(&self.repository)
    }

    fn set_keys(&mut self, keys: HashSet<String>) {
        let mut keys_string = String::new();
        keys.into_iter().for_each(|v| keys_string += &(v + ","));
        let keys_string = keys_string.trim_end_matches(',');

        let _ = self.repository.lock().unwrap().insert("keys", keys_string);
    }

    fn add_key(&mut self, key: String) {
        let mut keys = self.get_all_keys();

        keys.insert(key);

        self.set_keys(keys);
    }

    fn remove_key(&mut self, key: &str) {
        let mut keys = self.get_all_keys();

        keys.remove(key);

        self.set_keys(keys);
    }

    fn parse_pair_tuple(pair_tuple_text_view: &str) -> (String, String) {
        let parts: Vec<&str> = pair_tuple_text_view.split('_').collect();

        (parts[0].to_string(), parts[1].to_string())
    }

    fn parse_primary_from_ivec(key: vsdbsled::IVec) -> TimestampAndPairTuple {
        let key = str::from_utf8(&key.to_vec()).unwrap().to_string();

        let parts: Vec<&str> = key.rsplit("__").collect();
        let date_time = date_time_from_timestamp(parts[0].parse().unwrap());
        let pair_tuple = Self::parse_pair_tuple(parts[1]);

        (date_time, pair_tuple)
    }
}

impl Repository<TimestampAndPairTuple, f64> for F64ByTimestampAndPairTupleSled {
    // fn get_my_keys(&self) -> HashSet<TimestampAndPairTuple> {}

    fn read(&self, primary: TimestampAndPairTuple) -> Result<Option<f64>, String> {
        let key = self.stringify_primary(primary);

        self.repository
            .lock()
            .unwrap()
            .get(key)
            .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
            .map_err(|e| e.to_string())
    }

    fn read_range(
        &self,
        primary_from: TimestampAndPairTuple,
        primary_to: TimestampAndPairTuple,
    ) -> Result<HashMap<TimestampAndPairTuple, f64>, String> {
        let key_from = self.stringify_primary(primary_from);
        let key_to = self.stringify_primary(primary_to);

        let mut oks = HashMap::new();
        let mut errors = Vec::new();

        self.repository
            .lock()
            .unwrap()
            .range(key_from..key_to)
            .for_each(|v| match v {
                Ok((k, v)) => {
                    oks.insert(k, v);
                }
                Err(e) => errors.push(e.to_string()),
            });

        if !errors.is_empty() {
            let mut error_string = String::new();

            for error in errors {
                error_string += &(error + ". ");
            }

            Err(error_string)
        } else {
            let res = oks
                .into_iter()
                .map(|(k, v)| {
                    (
                        Self::parse_primary_from_ivec(k),
                        f64::from_ne_bytes(v[0..8].try_into().unwrap()),
                    )
                })
                .collect();

            Ok(res)
        }
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

            let key = self.stringify_primary(primary);

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
        let key = self.stringify_primary(primary);

        self.remove_key(&key);
        let _ = self.repository.lock().unwrap().remove(key);
        let _ = self.repository.lock().unwrap().flush();
    }
}
