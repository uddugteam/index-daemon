use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::helper_functions::date_time_subtract_sec;
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Debug, Clone)]
enum SubscriberType {
    System,
    Users(HashSet<CJ>),
}

impl SubscriberType {
    pub fn add(&mut self, subscriber: CJ) {
        match self {
            SubscriberType::System => {}
            SubscriberType::Users(subscribers) => {
                subscribers.insert(subscriber);
            }
        }
    }

    pub fn remove(&mut self, subscriber: &CJ) -> bool {
        match self {
            SubscriberType::System => false,
            SubscriberType::Users(subscribers) => {
                subscribers.remove(subscriber);

                subscribers.is_empty()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PercentChange {
    value: Option<f64>,
    percent_change: Option<f64>,
    timestamp: Option<DateTime<Utc>>,
    subscribers: SubscriberType,
}

impl PercentChange {
    pub async fn new_permanent(
        repository: Option<RepositoryForF64ByTimestamp>,
        percent_change_interval_sec: u64,
    ) -> Self {
        let debug = repository.is_some();
        if debug {
            debug!(
                "fn new_permanent BEGIN. percent_change_interval_sec: {}",
                percent_change_interval_sec,
            );
        }

        let (value, percent_change, timestamp) =
            Self::from_historical_inner(repository, percent_change_interval_sec).await;

        if debug {
            debug!(
                "fn new_permanent END. percent_change_interval_sec: {}",
                percent_change_interval_sec,
            );
        }

        Self {
            value,
            percent_change,
            timestamp,
            subscribers: SubscriberType::System,
        }
    }

    async fn from_historical_inner(
        repository: Option<RepositoryForF64ByTimestamp>,
        percent_change_interval_sec: u64,
    ) -> (Option<f64>, Option<f64>, Option<DateTime<Utc>>) {
        if let Some(repository) = repository {
            let to = Utc::now();
            let from = date_time_subtract_sec(to, percent_change_interval_sec);
            let primary = from..to;

            match repository.read_range(&primary).await {
                Ok(values) => match values.len() {
                    0 => (None, None, None),
                    // TODO: Check - possible bug - will lead to jump of percent change
                    1 => (Some(values[0].1), None, Some(to)),
                    _ => {
                        let first_value = values.first().unwrap().1;
                        let last_value = values.last().unwrap().1;

                        let percent_change = 100.0 * (last_value - first_value) / first_value;

                        (Some(last_value), Some(percent_change), Some(to))
                    }
                },
                Err(e) => panic!("Read from db error: {}", e),
            }
        } else {
            (None, None, None)
        }
    }

    pub async fn from_historical(
        repository: Option<RepositoryForF64ByTimestamp>,
        percent_change_interval_sec: u64,
        subscriber: CJ,
    ) -> Self {
        let (value, percent_change, timestamp) =
            Self::from_historical_inner(repository, percent_change_interval_sec).await;

        Self {
            value,
            percent_change,
            timestamp,
            subscribers: SubscriberType::Users(HashSet::from([subscriber])),
        }
    }

    pub fn set(&mut self, new_value: f64, timestamp: DateTime<Utc>) {
        self.percent_change = self
            .value
            .map(|old_value| 100.0 * (new_value - old_value) / old_value)
            .map(|value| (value * 100.0).round() / 100.0);

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
        self.subscribers.add(subscriber);
    }

    pub fn remove_subscriber(&mut self, subscriber: &CJ) -> bool {
        self.subscribers.remove(subscriber)
    }
}
