use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone)]
pub struct PercentChangeByInterval(HashMap<u64, Option<PercentChange>>);

impl PercentChangeByInterval {
    pub fn new_with(percent_change_interval_sec: u64) -> Self {
        Self(HashMap::from([(percent_change_interval_sec, None)]))
    }

    pub fn add_percent_change_interval(&mut self, percent_change_interval_sec: u64) {
        self.0.entry(percent_change_interval_sec).or_insert(None);
    }

    pub fn set(&mut self, value: f64, timestamp: DateTime<Utc>) {
        let timestamp_new_sec = timestamp.timestamp() as u64;

        for (&interval_sec, percent_change) in &mut self.0 {
            if let Some(percent_change) = percent_change {
                let timestamp_old_sec = percent_change.get_timestamp().timestamp() as u64;

                if timestamp_new_sec - timestamp_old_sec >= interval_sec {
                    // It's time to recalculate percent

                    percent_change.set(value, timestamp);
                } else {
                    // Too early
                }
            } else {
                *percent_change = Some(PercentChange::new(value, timestamp));
            }
        }
    }

    pub fn get_percent_change(&self, percent_change_interval_sec: u64) -> Option<f64> {
        if let Some(Some(percent_change)) = self.0.get(&percent_change_interval_sec) {
            percent_change.get_percent_change()
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub struct PercentChange {
    value: f64,
    percent_change: Option<f64>,
    timestamp: DateTime<Utc>,
}

impl PercentChange {
    pub fn new(value: f64, timestamp: DateTime<Utc>) -> Self {
        Self {
            value,
            percent_change: None,
            timestamp,
        }
    }

    pub fn set(&mut self, value: f64, timestamp: DateTime<Utc>) {
        let new_value = value;
        let old_value = self.value;

        self.percent_change = Some(100.0 * (new_value - old_value) / old_value);
        self.value = value;
        self.timestamp = timestamp;
    }

    pub fn get_percent_change(&self) -> Option<f64> {
        self.percent_change
    }

    pub fn get_timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
