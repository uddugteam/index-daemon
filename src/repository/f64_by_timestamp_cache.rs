use crate::repository::repository::Repository;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, min_date_time};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct F64ByTimestampCache {
    entity_name: String,
    repository: Arc<RwLock<HashMap<String, f64>>>,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampCache {
    pub fn new(
        entity_name: String,
        repository: Arc<RwLock<HashMap<String, f64>>>,
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

    async fn get_keys_by_range(&self, primary: Range<String>) -> Vec<String> {
        self.repository
            .read()
            .await
            .keys()
            .filter(|v| v >= &&primary.start && v <= &&primary.end)
            .cloned()
            .collect()
    }

    fn date_time_from_timestamp_millis(timestamp_millis: u64) -> DateTime<Utc> {
        date_time_from_timestamp_sec(timestamp_millis / 1000)
    }

    fn parse_primary_from_string(key: String) -> DateTime<Utc> {
        let parts: Vec<&str> = key.rsplit("__").collect();

        Self::date_time_from_timestamp_millis(parts.first().unwrap().parse().unwrap())
    }
}

#[async_trait]
impl Repository<DateTime<Utc>, f64> for F64ByTimestampCache {
    async fn read(&self, primary: DateTime<Utc>) -> Result<Option<f64>, String> {
        let key = self.stringify_primary(primary);

        Ok(self.repository.read().await.get(&key).cloned())
    }

    async fn read_range(
        &self,
        primary: Range<DateTime<Utc>>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let key_from = self.stringify_primary(primary.start);
        let key_to = self.stringify_primary(primary.end);

        let mut res = Vec::new();

        let repository = self.repository.read().await;

        for key in self.get_keys_by_range(key_from..key_to).await {
            let item = *repository.get(&key).unwrap();
            let key = Self::parse_primary_from_string(key);

            res.push((key, item));
        }
        res.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(res)
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

            let _ = self.repository.write().await.insert(key, new_value);

            Some(Ok(()))
        } else {
            // Too early
            None
        }
    }

    async fn delete(&mut self, primary: DateTime<Utc>) {
        let key = self.stringify_primary(primary);

        let _ = self.repository.write().await.remove(&key);
    }

    async fn delete_multiple(&mut self, primary: &[DateTime<Utc>]) {
        for &key in primary {
            let key = self.stringify_primary(key);

            let _ = self.repository.write().await.remove(&key);
        }
    }
}
