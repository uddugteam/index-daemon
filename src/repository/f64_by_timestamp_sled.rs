use crate::repository::repository::Repository;
use crate::worker::helper_functions::{datetime_from_timestamp_millis, min_datetime};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::ops::Range;
use std::str;

#[derive(Clone)]
pub struct F64ByTimestampSled {
    entity_name: String,
    repository: vsdbsled::Db,
    frequency_ms: u64,
    last_insert_timestamp: DateTime<Utc>,
}

impl F64ByTimestampSled {
    pub fn new(entity_name: String, repository: vsdbsled::Db, frequency_ms: u64) -> Self {
        Self {
            entity_name,
            repository,
            frequency_ms,
            last_insert_timestamp: min_datetime(),
        }
    }

    fn stringify_primary(&self, primary: &DateTime<Utc>) -> String {
        format!("{}__{}", self.entity_name, primary.timestamp_millis())
    }

    fn parse_primary_from_ivec(key: vsdbsled::IVec) -> DateTime<Utc> {
        let key = str::from_utf8(&key).unwrap().to_string();

        let parts: Vec<&str> = key.rsplit("__").collect();

        datetime_from_timestamp_millis(parts.first().unwrap().parse().unwrap())
    }
}

#[async_trait]
impl Repository<DateTime<Utc>, f64> for F64ByTimestampSled {
    async fn read(&self, primary: &DateTime<Utc>) -> Result<Option<f64>, String> {
        let key = self.stringify_primary(primary);

        self.repository
            .get(key)
            .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
            .map_err(|e| e.to_string())
    }

    async fn read_range(
        &self,
        primary: &Range<DateTime<Utc>>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let key_from = self.stringify_primary(&primary.start);
        let key_to = self.stringify_primary(&primary.end);

        let (oks, errors): (Vec<_>, Vec<_>) = self
            .repository
            .range(key_from..key_to)
            .into_iter()
            .partition(Result::is_ok);
        let oks = oks.into_iter().map(Result::unwrap);
        let mut errors = errors.into_iter().map(Result::unwrap_err);

        if let Some(error) = errors.next() {
            Err(error.to_string())
        } else {
            let mut res: Vec<(DateTime<Utc>, f64)> = oks
                .map(|(k, v)| {
                    (
                        Self::parse_primary_from_ivec(k),
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

            let key = self.stringify_primary(&primary);

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

    async fn delete(&mut self, primary: &DateTime<Utc>) {
        let key = self.stringify_primary(primary);

        let _ = self.repository.remove(key);
        let _ = self.repository.flush();
    }

    async fn delete_multiple(&mut self, primary: &[DateTime<Utc>]) {
        for key in primary {
            let key = self.stringify_primary(key);

            let _ = self.repository.remove(key);
        }

        let _ = self.repository.flush();
    }
}
