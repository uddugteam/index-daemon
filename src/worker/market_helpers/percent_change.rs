use crate::config_scheme::config_scheme::ConfigScheme;
use crate::worker::helper_functions::{min_date_time};
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct PercentChangeByInterval(HashMap<u64, PercentChange>);

impl PercentChangeByInterval {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add_percent_change_interval(
        &mut self,
        percent_change_interval_sec: u64,
        subscriber: CJ,
    ) {
        self.0
            .entry(percent_change_interval_sec)
            .or_default()
            .add_subscriber(subscriber);
    }

    pub fn remove_percent_change_interval(&mut self, subscriber: &CJ) {
        let mut percent_change_intervals_to_remove = Vec::new();

        for (percent_change_interval, percent_change) in &mut self.0 {
            let is_empty = percent_change.remove_subscriber(subscriber);

            if is_empty {
                percent_change_intervals_to_remove.push(*percent_change_interval);
            }
        }

        for percent_change_interval in percent_change_intervals_to_remove {
            self.0.remove(&percent_change_interval);
        }
    }

    pub fn set(&mut self, value: f64, timestamp: DateTime<Utc>) {
        let timestamp_new_sec = timestamp.timestamp() as u64;

        for (&interval_sec, percent_change) in &mut self.0 {
            let timestamp_old_sec = percent_change
                .get_timestamp()
                .unwrap_or(min_date_time())
                .timestamp() as u64;

            if timestamp_new_sec - timestamp_old_sec >= interval_sec {
                // It's time to recalculate percent

                percent_change.set(value, timestamp);
            } else {
                // Too early
            }
        }
    }

    pub fn get_percent_change(&self, percent_change_interval_sec: u64) -> Option<f64> {
        if let Some(percent_change) = self.0.get(&percent_change_interval_sec) {
            percent_change.get_percent_change()
        } else {
            None
        }
    }
}

impl From<ConfigScheme> for PercentChangeByInterval {
    fn from(_config: ConfigScheme) -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
struct PercentChange {
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
