use crate::worker::market_helpers::percent_change::PercentChange;
use crate::worker::market_helpers::percent_change_by_interval::PercentChangeByInterval;
use crate::worker::network_helpers::ws_server::holders::helper_functions::{
    HolderHashMap, HolderKey,
};
use crate::worker::network_helpers::ws_server::ws_channels::CJ;

#[derive(Clone)]
pub struct PercentChangeByIntervalHolder(HolderHashMap<PercentChangeByInterval>);

impl PercentChangeByIntervalHolder {
    pub fn new(percent_change_holder: HolderHashMap<PercentChangeByInterval>) -> Self {
        Self(percent_change_holder)
    }

    pub fn contains_holder_key(&self, holder_key: &HolderKey) -> bool {
        self.0.contains_key(holder_key)
    }

    pub async fn contains_interval(
        &self,
        holder_key: &HolderKey,
        percent_change_interval_sec: u64,
    ) -> Option<bool> {
        if let Some(percent_change_by_interval) = self.0.get(holder_key) {
            let res = percent_change_by_interval
                .read()
                .await
                .contains_interval(percent_change_interval_sec);

            Some(res)
        } else {
            None
        }
    }

    pub async fn add_interval(
        &self,
        holder_key: &HolderKey,
        percent_change_interval_sec: u64,
        percent_change: PercentChange,
    ) {
        if let Some(percent_change_by_interval) = self.0.get(holder_key) {
            percent_change_by_interval
                .write()
                .await
                .add_interval(percent_change_interval_sec, percent_change);
        }
    }

    pub async fn add_subscriber(
        &self,
        holder_key: &HolderKey,
        percent_change_interval_sec: u64,
        subscriber: CJ,
    ) {
        if let Some(percent_change_by_interval) = self.0.get(holder_key) {
            percent_change_by_interval
                .write()
                .await
                .add_subscriber(percent_change_interval_sec, subscriber);
        }
    }

    pub async fn remove(&mut self, subscriber: &CJ) {
        for percent_change in self.0.values() {
            percent_change.write().await.remove_interval(subscriber);
        }
    }
}
