use crate::repository::repository::Repository;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, min_date_time};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use std::ops::Range;
use std::str;

const DAY_IN_SECONDS: u64 = 86_400;

#[derive(Clone)]
pub struct F64ByTimestampSled {
    entity_name: String,
    repository: vsdbsled::Db,
    cache: MultiplexedConnection,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampSled {
    pub fn new(
        entity_name: String,
        repository: vsdbsled::Db,
        cache: MultiplexedConnection,
        frequency_ms: u64,
    ) -> Self {
        Self {
            entity_name,
            repository,
            cache,
            frequency_ms,
            last_insert_timestamp: min_date_time(),
        }
    }

    fn stringify_primary(&self, primary: DateTime<Utc>) -> String {
        format!("{}__{}", self.entity_name, primary.timestamp_millis())
    }

    fn date_time_from_timestamp_millis(timestamp_millis: u64) -> DateTime<Utc> {
        date_time_from_timestamp_sec(timestamp_millis / 1000)
    }

    fn parse_primary_from_string(key: &str) -> DateTime<Utc> {
        let parts: Vec<&str> = key.rsplit("__").collect();

        Self::date_time_from_timestamp_millis(parts.first().unwrap().parse().unwrap())
    }

    fn parse_primary_from_ivec(key: vsdbsled::IVec) -> DateTime<Utc> {
        let key = str::from_utf8(&key).unwrap().to_string();

        Self::parse_primary_from_string(&key)
    }

    async fn cache_read(
        mut cache: MultiplexedConnection,
        key: &str,
    ) -> Result<Option<f64>, String> {
        cache.get(&key).await.map_err(|e| e.to_string())
    }

    fn cache_check_interval(
        key_range: Range<String>,
        values: &[(DateTime<Utc>, f64)],
    ) -> Result<(), String> {
        let key_from = Self::parse_primary_from_string(&key_range.start);
        let key_to = Self::parse_primary_from_string(&key_range.end);

        let not_enough_from =
            (values[0].0 - key_from).num_seconds().unsigned_abs() > DAY_IN_SECONDS;

        let not_enough_to = (values.last().unwrap().0 - key_to)
            .num_seconds()
            .unsigned_abs()
            > DAY_IN_SECONDS;

        if not_enough_from || not_enough_to {
            Err(format!(
                "Redis: Cache does not contain the entire interval: {:?}.",
                key_range,
            ))
        } else {
            Ok(())
        }
    }

    async fn cache_read_range(
        mut cache: MultiplexedConnection,
        key_range: Range<String>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let keys: Vec<String> = cache
            .scan()
            .await
            .map_err(|e| e.to_string())?
            .filter(|v| futures::future::ready(v >= &key_range.start))
            .filter(|v| futures::future::ready(v <= &key_range.end))
            .collect()
            .await;

        // // Sorted after function call
        // keys.sort();

        if !keys.is_empty() {
            let mut res = Vec::new();

            for key in keys {
                let value: Option<f64> = cache.get(&key).await.map_err(|e| e.to_string())?;
                let value = value.ok_or(format!(
                    "Redis: Read from cache error: Value with key \"{}\" not found.",
                    key
                ))?;

                let key = Self::parse_primary_from_string(&key);

                res.push((key, value));
            }

            Self::cache_check_interval(key_range, &res)?;

            Ok(res)
        } else {
            Err(format!(
                "Redis: No values found in the interval {:?}.",
                key_range,
            ))
        }
    }

    async fn cache_set(
        mut cache: MultiplexedConnection,
        key: String,
        value: f64,
    ) -> Result<(), String> {
        let res = cache
            .set::<_, _, ()>(&key, value)
            .await
            .map_err(|e| e.to_string());

        if res.is_err() {
            error!("Redis: Write to cache error: {:?}", res);
        }

        res
    }

    async fn cache_set_mul(
        mut cache: MultiplexedConnection,
        values: Vec<(String, f64)>,
    ) -> Result<(), String> {
        for (key, value) in values {
            let res = cache
                .set::<_, _, ()>(&key, value)
                .await
                .map_err(|e| e.to_string());

            if res.is_err() {
                error!("Redis: Write to cache error: {:?}", res);

                return res;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Repository<DateTime<Utc>, f64> for F64ByTimestampSled {
    async fn read(&self, primary: DateTime<Utc>) -> Result<Option<f64>, String> {
        let key = self.stringify_primary(primary);

        let res = Self::cache_read(self.cache.clone(), &key).await;

        if res.is_ok() {
            res
        } else {
            let res = self
                .repository
                .get(&key)
                .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
                .map_err(|e| e.to_string());

            if let Ok(Some(value)) = res {
                tokio::spawn(Self::cache_set(self.cache.clone(), key, value));
            } else {
                error!("Sled: Read from repository error: {:#?}", res);
            }

            res
        }
    }

    async fn read_range(
        &self,
        primary: Range<DateTime<Utc>>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let key_from = self.stringify_primary(primary.start);
        let key_to = self.stringify_primary(primary.end);
        let range = key_from..key_to;

        let mut res_vec =
            if let Ok(res) = Self::cache_read_range(self.cache.clone(), range.clone()).await {
                res
            } else {
                let mut res_vec = Vec::new();

                for item in self.repository.range(range) {
                    let (k, v) = item.map_err(|e| e.to_string())?;

                    res_vec.push((
                        Self::parse_primary_from_ivec(k),
                        f64::from_ne_bytes(v[0..8].try_into().unwrap()),
                    ));
                }

                let cache_new_values = res_vec
                    .iter()
                    .map(|(k, v)| (self.stringify_primary(*k), *v))
                    .collect();

                tokio::spawn(Self::cache_set_mul(self.cache.clone(), cache_new_values));

                res_vec
            };
        res_vec.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(res_vec)
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

            tokio::spawn(Self::cache_set(self.cache.clone(), key.clone(), new_value));

            let res = self
                .repository
                .insert(key, new_value.to_ne_bytes())
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = self.repository.flush();

            Some(res)
        } else {
            // Too early
            None
        }
    }

    async fn delete(&mut self, primary: DateTime<Utc>) {
        let key = self.stringify_primary(primary);

        let _ = self.repository.remove(key);
        let _ = self.repository.flush();
    }

    async fn delete_multiple(&mut self, primary: &[DateTime<Utc>]) {
        for &key in primary {
            let key = self.stringify_primary(key);

            let _ = self.repository.remove(key);
        }

        let _ = self.repository.flush();
    }
}
