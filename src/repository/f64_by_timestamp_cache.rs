use crate::repository::repository::Repository;
use crate::worker::helper_functions::{
    datetime_from_timestamp_millis, datetime_from_timestamp_sec, min_datetime,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use tokio::sync::RwLock;

// 1 year in seconds
const STORE_RANGE_SEC: u64 = 31_536_000;

#[derive(Clone)]
pub struct F64ByTimestampCache {
    entity_name: Option<String>,
    repository: Arc<RwLock<HashMap<String, f64>>>,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampCache {
    pub fn new(
        entity_name: Option<String>,
        repository: Arc<RwLock<HashMap<String, f64>>>,
        frequency_ms: u64,
    ) -> Self {
        Self {
            entity_name,
            repository,
            frequency_ms,
            last_insert_timestamp: min_datetime(),
        }
    }

    fn stringify_primary(&self, primary: &DateTime<Utc>) -> String {
        let t = primary.timestamp_millis();

        self.entity_name
            .as_ref()
            .map_or(t.to_string(), |v| format!("{}__{}", v, t))
    }

    fn check_primary_range(primary: &Range<DateTime<Utc>>) -> bool {
        Self::check_primary(&primary.start) && Self::check_primary(&primary.end)
    }

    fn check_primary(primary: &DateTime<Utc>) -> bool {
        let now_timestamp_sec = Utc::now().timestamp() as u64;
        let earliest_timestamp = datetime_from_timestamp_sec(now_timestamp_sec - STORE_RANGE_SEC);

        primary >= &earliest_timestamp
    }

    async fn get_keys_by_range(&self, primary: &Range<DateTime<Utc>>) -> Vec<String> {
        let key_from = self.stringify_primary(&primary.start);
        let key_to = self.stringify_primary(&primary.end);
        let key_range = key_from..key_to;

        self.repository
            .read()
            .await
            .keys()
            .filter(|&key| key_range.contains(key))
            .cloned()
            .collect()
    }

    fn parse_primary_from_string(key: String) -> DateTime<Utc> {
        let parts: Vec<&str> = key.rsplit("__").collect();

        datetime_from_timestamp_millis(parts.first().unwrap().parse().unwrap())
    }
}

#[async_trait]
impl Repository<DateTime<Utc>, f64> for F64ByTimestampCache {
    async fn read(&self, primary: &DateTime<Utc>) -> Result<Option<f64>, String> {
        let key = self.stringify_primary(primary);

        Ok(self.repository.read().await.get(&key).cloned())
    }

    async fn read_range(
        &self,
        primary: &Range<DateTime<Utc>>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let mut res = Vec::new();

        if Self::check_primary_range(primary) {
            let repository = self.repository.read().await;

            for key in self.get_keys_by_range(primary).await {
                let item = *repository.get(&key).unwrap();
                let key = Self::parse_primary_from_string(key);

                res.push((key, item));
            }
            res.sort_by(|a, b| a.0.cmp(&b.0));
        }

        Ok(res)
    }

    async fn insert(
        &mut self,
        primary: DateTime<Utc>,
        new_value: f64,
    ) -> Option<Result<(), String>> {
        let time_passed_ms = (primary - self.last_insert_timestamp).num_milliseconds() as u64;

        if (time_passed_ms > self.frequency_ms) && Self::check_primary(&primary) {
            // Enough time passed AND `primary` is fresh enough
            self.last_insert_timestamp = primary;

            let key = self.stringify_primary(&primary);

            let _ = self.repository.write().await.insert(key, new_value);

            Some(Ok(()))
        } else {
            // Too early
            None
        }
    }

    async fn delete(&mut self, primary: &DateTime<Utc>) {
        let key = self.stringify_primary(primary);

        let _ = self.repository.write().await.remove(&key);
    }

    async fn delete_multiple(&mut self, primary: &[DateTime<Utc>]) {
        for key in primary {
            self.delete(key).await;
        }
    }
}
