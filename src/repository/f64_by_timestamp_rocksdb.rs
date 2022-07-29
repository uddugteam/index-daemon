use crate::repository::repository::Repository;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, min_date_time};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::ops::Range;
use std::str;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct F64ByTimestampRocksdb {
    entity_name: String,
    repository: Arc<RwLock<rocksdb::DB>>,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampRocksdb {
    pub fn new(
        entity_name: String,
        repository: Arc<RwLock<rocksdb::DB>>,
        frequency_ms: u64,
    ) -> Self {
        Self {
            entity_name,
            repository,
            frequency_ms,
            last_insert_timestamp: min_date_time(),
        }
    }

    fn stringify_primary(&self, primary: DateTime<Utc>) -> String {
        format!("{}__{}", self.entity_name, primary.timestamp_millis())
    }

    async fn get_keys_by_range(&self, primary: Range<DateTime<Utc>>) -> Vec<String> {
        let key_from = self.stringify_primary(primary.start);
        let key_to = self.stringify_primary(primary.end);
        let key_range = key_from..key_to;

        self.repository
            .read()
            .await
            .iterator(rocksdb::IteratorMode::Start)
            .map(|(key, _value)| {
                // Drop value
                key
            })
            .map(|key| str::from_utf8(&key).unwrap().to_string())
            .filter(|key| key_range.contains(key))
            .collect()
    }

    fn date_time_from_timestamp_millis(timestamp_millis: u64) -> DateTime<Utc> {
        date_time_from_timestamp_sec(timestamp_millis / 1000)
    }

    fn parse_primary_from_string(key_string: &str) -> DateTime<Utc> {
        let parts: Vec<&str> = key_string.rsplit("__").collect();

        Self::date_time_from_timestamp_millis(parts.first().unwrap().parse().unwrap())
    }
}

#[async_trait]
impl Repository<DateTime<Utc>, f64> for F64ByTimestampRocksdb {
    async fn read(&self, primary: DateTime<Utc>) -> Result<Option<f64>, String> {
        let key = self.stringify_primary(primary);

        self.repository
            .read()
            .await
            .get(key)
            .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
            .map_err(|e| e.to_string())
    }

    async fn read_range(
        &self,
        primary: Range<DateTime<Utc>>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let keys = self.get_keys_by_range(primary).await;

        let repository = self.repository.read().await;

        let (oks, errors): (Vec<_>, Vec<_>) = keys
            .into_iter()
            .map(|key| repository.get(&key).map(|v| v.map(|v| (key, v))))
            .partition(Result::is_ok);
        let oks = oks.into_iter().filter_map(Result::unwrap);
        let mut errors = errors.into_iter().map(Result::unwrap_err);

        if let Some(error) = errors.next() {
            Err(error.to_string())
        } else {
            let mut res: Vec<(DateTime<Utc>, f64)> = oks
                .map(|(k, v)| {
                    (
                        Self::parse_primary_from_string(&k),
                        f64::from_ne_bytes(v[0..8].try_into().unwrap()),
                    )
                })
                .collect();
            res.sort_by(|a, b| a.0.cmp(&b.0));

            Ok(res)
        }
    }

    async fn insert(
        &mut self,
        primary: DateTime<Utc>,
        new_value: f64,
    ) -> Option<Result<(), String>> {
        if (primary - self.last_insert_timestamp).num_milliseconds() as u64 > self.frequency_ms {
            // Enough time passed
            self.last_insert_timestamp = primary;

            let key = self.stringify_primary(primary);

            let res = self
                .repository
                .read()
                .await
                .put(key, new_value.to_ne_bytes())
                .map(|_| ())
                .map_err(|e| e.to_string());

            Some(res)
        } else {
            // Too early
            None
        }
    }

    async fn delete(&mut self, primary: DateTime<Utc>) {
        let key = self.stringify_primary(primary);

        let _ = self.repository.read().await.delete(key);
    }

    async fn delete_multiple(&mut self, primary: &[DateTime<Utc>]) {
        for &key in primary {
            self.delete(key).await;
        }
    }
}
