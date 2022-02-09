use crate::repository::f64_by_timestamp_and_pair_tuple_sled::TimestampAndPairTuple;
use crate::worker::network_helpers::ws_server::hepler_functions::thin_by_interval;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_request::Interval;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct CoinAveragePriceHistoricalSnapshots(Vec<CoinAveragePriceHistoricalSnapshot>);

impl CoinAveragePriceHistoricalSnapshots {
    pub fn with_interval(res: HashMap<TimestampAndPairTuple, f64>, interval: Interval) -> Self {
        let res = res.into_iter().map(|(k, v)| (k.0, v)).collect();
        let res = thin_by_interval(res, interval);
        let res = res
            .into_iter()
            .map(|(timestamp, value)| CoinAveragePriceHistoricalSnapshot { value, timestamp })
            .collect();

        Self(res)
    }
}

#[derive(Serialize, Clone)]
pub struct CoinAveragePriceHistoricalSnapshot {
    value: f64,
    #[serde(with = "ser_date_into_timestamp")]
    timestamp: DateTime<Utc>,
}
