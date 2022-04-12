use crate::worker::market_helpers::percent_change::PercentChangeByInterval;
use crate::worker::network_helpers::ws_server::holders::helper_functions::{
    HolderHashMap, HolderKey,
};

#[derive(Clone)]
pub struct PercentChangeByIntervalHolder(HolderHashMap<PercentChangeByInterval>);

/// TODO: Add percent change interval removal
impl PercentChangeByIntervalHolder {
    pub fn new(percent_change_holder: HolderHashMap<PercentChangeByInterval>) -> Self {
        Self(percent_change_holder)
    }

    pub fn contains_key(&self, key: &HolderKey) -> bool {
        self.0.contains_key(key)
    }

    pub async fn add(&self, holder_key: &HolderKey, percent_change_interval_sec: u64) {
        if let Some(percent_change) = self.0.get(holder_key) {
            percent_change
                .write()
                .await
                .add_percent_change_interval(percent_change_interval_sec);
        }
    }

    // TODO: Add percent change interval removal
}
