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

    pub fn contains_key(&self, key: &HolderKey) -> bool {
        self.0.contains_key(key)
    }

    pub async fn add(
        &self,
        holder_key: &HolderKey,
        percent_change_interval_sec: u64,
        subscriber: CJ,
    ) {
        if let Some(percent_change) = self.0.get(holder_key) {
            percent_change
                .write()
                .await
                .add_percent_change_interval(percent_change_interval_sec, subscriber);
        }
    }

    pub async fn remove(&mut self, subscriber: &CJ) {
        for percent_change in self.0.values() {
            percent_change
                .write()
                .await
                .remove_percent_change_interval(subscriber);
        }
    }
}
