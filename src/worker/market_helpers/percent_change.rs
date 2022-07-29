use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::helper_functions::{date_time_from_timestamp_sec, date_time_subtract_sec};
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

const OVERLAP_SEC: u64 = 1;

#[derive(Copy, Clone)]
enum RoughFetchDirection {
    NearestPrevious,
    NearestNext,
}

#[derive(Debug, Clone)]
pub struct PercentChange {
    value: Option<f64>,
    percent_change: Option<f64>,
    timestamp: Option<DateTime<Utc>>,
    subscribers: HashSet<CJ>,
}

impl PercentChange {
    pub async fn from_historical(
        repository: Option<RepositoryForF64ByTimestamp>,
        percent_change_interval_sec: u64,
        subscriber: CJ,
    ) -> Self {
        let (value, percent_change, timestamp) =
            Self::get_from_db(repository, percent_change_interval_sec).await;

        Self {
            value,
            percent_change,
            timestamp,
            subscribers: HashSet::from([subscriber]),
        }
    }

    async fn get_from_db(
        repository: Option<RepositoryForF64ByTimestamp>,
        percent_change_interval_sec: u64,
    ) -> (Option<f64>, Option<f64>, Option<DateTime<Utc>>) {
        let (value, percent_change, timestamp) = if let Some(repository) = repository {
            let timestamp_to = Utc::now();
            let timestamp_from = date_time_subtract_sec(timestamp_to, percent_change_interval_sec);

            let value_from = Self::get_one_value_from_db(
                &repository,
                timestamp_from,
                RoughFetchDirection::NearestNext,
            )
            .await;
            let value_to = Self::get_one_value_from_db(
                &repository,
                timestamp_to,
                RoughFetchDirection::NearestPrevious,
            )
            .await;

            match (value_from, value_to) {
                (Some((_timestamp_from, value_from)), Some((timestamp_to, value_to))) => {
                    let percent_change = 100.0 * (value_to - value_from) / value_from;

                    (Some(value_to), Some(percent_change), Some(timestamp_to))
                }
                (Some((timestamp_from, value_from)), None) => {
                    // TODO: Check - possible bug - will lead to jump of percent change

                    (Some(value_from), None, Some(timestamp_from))
                }
                (None, Some((timestamp_to, value_to))) => {
                    // TODO: Check - possible bug - will lead to jump of percent change

                    (Some(value_to), None, Some(timestamp_to))
                }
                (None, None) => (None, None, None),
            }
        } else {
            (None, None, None)
        };

        (value, percent_change, timestamp)
    }

    async fn get_one_value_from_db(
        repository: &RepositoryForF64ByTimestamp,
        key: DateTime<Utc>,
        direction: RoughFetchDirection,
    ) -> Option<(DateTime<Utc>, f64)> {
        let (from, to) = match direction {
            RoughFetchDirection::NearestPrevious => {
                let timestamp_from = key.timestamp() as u64 - OVERLAP_SEC;
                let key_from = date_time_from_timestamp_sec(timestamp_from);

                (key_from, key)
            }
            RoughFetchDirection::NearestNext => {
                let timestamp_to = key.timestamp() as u64 + OVERLAP_SEC;
                let key_to = date_time_from_timestamp_sec(timestamp_to);

                (key, key_to)
            }
        };

        match repository.read_range(from..to).await {
            Ok(values) => match direction {
                RoughFetchDirection::NearestPrevious => values.last().cloned(),
                RoughFetchDirection::NearestNext => values.first().cloned(),
            },
            Err(e) => panic!("Read from db error: {}", e),
        }
    }

    pub fn set(&mut self, new_value: f64, timestamp: DateTime<Utc>) {
        self.percent_change = self
            .value
            .map(|old_value| 100.0 * (new_value - old_value) / old_value);

        self.value = Some(new_value);
        self.timestamp = Some(timestamp);
    }

    pub fn get_percent_change(&self) -> Option<f64> {
        self.percent_change
    }

    pub fn get_timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp
    }

    pub fn add_subscriber(&mut self, subscriber: CJ) {
        self.subscribers.insert(subscriber);
    }

    pub fn remove_subscriber(&mut self, subscriber: &CJ) -> bool {
        self.subscribers.remove(subscriber);

        self.subscribers.is_empty()
    }
}
