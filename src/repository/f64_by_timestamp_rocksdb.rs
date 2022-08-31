use crate::repository::f64_by_timestamp_cache::F64ByTimestampCache;
use crate::repository::repository::Repository;
use crate::worker::helper_functions::{date_time_from_timestamp_millis, min_date_time};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct F64ByTimestampRocksdb {
    repository: Arc<RwLock<rocksdb::DB>>,
    cache: Option<F64ByTimestampCache>,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampRocksdb {
    pub fn new(
        repository: Arc<RwLock<rocksdb::DB>>,
        cache: Option<Arc<RwLock<HashMap<String, f64>>>>,
        frequency_ms: u64,
    ) -> Self {
        Self {
            repository,
            cache: cache.map(|cache| F64ByTimestampCache::new(None, cache, frequency_ms)),
            frequency_ms,
            last_insert_timestamp: min_date_time(),
        }
    }

    async fn get_keys_by_range(&self, primary: &Range<DateTime<Utc>>) -> Vec<DateTime<Utc>> {
        let key_from = primary.start.timestamp_millis() as u64;
        let key_to = primary.end.timestamp_millis() as u64;
        let key_range = key_from..key_to;

        self.repository
            .read()
            .await
            .iterator(rocksdb::IteratorMode::Start)
            .map(|(key, _value)| {
                // Drop value
                key
            })
            .map(|key| u64::from_ne_bytes(key.as_ref().try_into().unwrap()))
            .filter(|key| key_range.contains(key))
            .map(date_time_from_timestamp_millis)
            .collect()
    }
}

#[async_trait]
impl Repository<DateTime<Utc>, f64> for F64ByTimestampRocksdb {
    async fn read(&self, primary: &DateTime<Utc>) -> Result<Option<f64>, String> {
        // async map
        let res = if let Some(cache) = &self.cache {
            Some(cache.read(primary).await)
        } else {
            None
        };

        if let Some(Ok(Some(res))) = res {
            Ok(Some(res))
        } else {
            let key = primary.timestamp_millis() as u64;

            self.repository
                .read()
                .await
                .get(key.to_ne_bytes())
                .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
                .map_err(|e| e.to_string())
        }
    }

    async fn read_range(
        &self,
        primary: &Range<DateTime<Utc>>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        // async map
        let res = if let Some(cache) = &self.cache {
            Some(cache.read_range(primary).await)
        } else {
            None
        };

        match res {
            Some(Ok(res)) if !res.is_empty() => Ok(res),
            _ => {
                let keys = self.get_keys_by_range(primary).await;

                let (oks, errors): (Vec<_>, Vec<_>) = {
                    let repository = self.repository.read().await;

                    keys.into_iter()
                        .map(|key| key.timestamp_millis() as u64)
                        .map(|key| key.to_ne_bytes())
                        .map(|key| repository.get(&key).map(|v| v.map(|v| (key, v))))
                        .partition(Result::is_ok)
                };
                let oks = oks.into_iter().filter_map(Result::unwrap);
                let mut errors = errors.into_iter().map(Result::unwrap_err);

                if let Some(error) = errors.next() {
                    Err(error.to_string())
                } else {
                    let mut res: Vec<(DateTime<Utc>, f64)> = oks
                        .map(|(k, v)| {
                            (
                                date_time_from_timestamp_millis(u64::from_ne_bytes(k)),
                                f64::from_ne_bytes(v[0..8].try_into().unwrap()),
                            )
                        })
                        .collect();
                    res.sort_by(|a, b| a.0.cmp(&b.0));

                    Ok(res)
                }
            }
        }
    }

    async fn insert(
        &mut self,
        primary: DateTime<Utc>,
        new_value: f64,
    ) -> Option<Result<(), String>> {
        if (primary - self.last_insert_timestamp).num_milliseconds() as u64 > self.frequency_ms {
            // Enough time passed

            // async map
            if let Some(cache) = &mut self.cache {
                cache.insert(primary, new_value).await;
            }

            self.last_insert_timestamp = primary;

            let key = primary.timestamp_millis() as u64;

            let res = self
                .repository
                .read()
                .await
                .put(key.to_ne_bytes(), new_value.to_ne_bytes())
                .map(|_| ())
                .map_err(|e| e.to_string());

            Some(res)
        } else {
            // Too early
            None
        }
    }

    async fn delete(&mut self, primary: &DateTime<Utc>) {
        let key = primary.timestamp_millis() as u64;

        let _ = self.repository.read().await.delete(key.to_ne_bytes());

        // async map
        if let Some(cache) = &mut self.cache {
            cache.delete(primary).await;
        }
    }

    async fn delete_multiple(&mut self, primary: &[DateTime<Utc>]) {
        let repository = self.repository.read().await;

        for key in primary {
            let key = key.timestamp_millis() as u64;

            let _ = repository.delete(key.to_ne_bytes());
        }
    }
}
