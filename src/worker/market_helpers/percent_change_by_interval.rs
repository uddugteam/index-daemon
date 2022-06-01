use crate::config_scheme::config_scheme::ConfigScheme;
use crate::worker::helper_functions::min_date_time;
use crate::worker::market_helpers::percent_change::PercentChange;
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone)]
pub struct PercentChangeByInterval(HashMap<u64, PercentChange>);

impl PercentChangeByInterval {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn contains_interval(&self, percent_change_interval_sec: u64) -> bool {
        self.0.contains_key(&percent_change_interval_sec)
    }

    pub fn add_interval(
        &mut self,
        percent_change_interval_sec: u64,
        percent_change: PercentChange,
    ) -> Option<()> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.0.entry(percent_change_interval_sec) {
            e.insert(percent_change);

            Some(())
        } else {
            None
        }
    }

    pub fn add_subscriber(
        &mut self,
        percent_change_interval_sec: u64,
        subscriber: CJ,
    ) -> Option<()> {
        if let Some(percent_change) = self.0.get_mut(&percent_change_interval_sec) {
            percent_change.add_subscriber(subscriber);

            Some(())
        } else {
            None
        }
    }

    pub fn remove_interval(&mut self, subscriber: &CJ) {
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
                .unwrap_or_else(min_date_time)
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
