use crate::config_scheme::async_from::AsyncFrom;
use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::RepositoryForF64ByTimestamp;
use crate::worker::defaults::PERMANENT_PERCENT_CHANGE_INTERVALS_SEC;
use crate::worker::helper_functions::min_date_time;
use crate::worker::market_helpers::percent_change::PercentChange;
use crate::worker::network_helpers::ws_server::ws_channels::CJ;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::{hash_map, HashMap};

#[derive(Clone)]
pub struct PercentChangeByInterval(HashMap<u64, PercentChange>);

impl PercentChangeByInterval {
    pub async fn new(repository: Option<RepositoryForF64ByTimestamp>) -> Self {
        let mut res = Self(HashMap::new());

        let mut futures = Vec::new();
        for percent_change_interval_sec in PERMANENT_PERCENT_CHANGE_INTERVALS_SEC {
            let repository_2 = repository.clone();
            let future = async move {
                (
                    percent_change_interval_sec,
                    PercentChange::new_permanent(repository_2, percent_change_interval_sec).await,
                )
            };
            futures.push(future);
        }

        let future_results = futures::future::join_all(futures).await;
        for (percent_change_interval_sec, percent_change) in future_results {
            res.add_interval(percent_change_interval_sec, percent_change);
        }

        res
    }

    pub fn contains_interval(&self, percent_change_interval_sec: u64) -> bool {
        self.0.contains_key(&percent_change_interval_sec)
    }

    pub fn add_interval(
        &mut self,
        percent_change_interval_sec: u64,
        percent_change: PercentChange,
    ) -> Option<()> {
        if let hash_map::Entry::Vacant(e) = self.0.entry(percent_change_interval_sec) {
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
        self.0
            .get_mut(&percent_change_interval_sec)
            .map(|percent_change| percent_change.add_subscriber(subscriber))
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

#[async_trait]
impl AsyncFrom<(ConfigScheme, Option<RepositoryForF64ByTimestamp>)> for PercentChangeByInterval {
    async fn from(
        (_config, repository): (ConfigScheme, Option<RepositoryForF64ByTimestamp>),
    ) -> Self {
        Self::new(repository).await
    }
}
