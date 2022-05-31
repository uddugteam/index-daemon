use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::helper_functions::date_time_subtract_sec;
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

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
        let (value, percent_change, timestamp) = if let Some(repository) = repository {
            let to = Utc::now();
            let from = date_time_subtract_sec(to, percent_change_interval_sec);

            match repository.read_range(from..to).await {
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
        };

        Self {
            value,
            percent_change,
            timestamp,
            subscribers: HashSet::from([subscriber]),
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
