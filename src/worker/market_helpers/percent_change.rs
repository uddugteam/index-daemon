use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct PercentChange {
    value: Option<f64>,
    percent_change: Option<f64>,
    timestamp: Option<DateTime<Utc>>,
    subscribers: HashSet<CJ>,
}

impl PercentChange {
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
