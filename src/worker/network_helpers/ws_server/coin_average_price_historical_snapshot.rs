use crate::worker::network_helpers::ws_server::hepler_functions::thin_by_interval;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_request::Interval;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
pub struct CoinAveragePriceHistoricalSnapshots(Vec<CoinAveragePriceHistoricalSnapshot>);

impl CoinAveragePriceHistoricalSnapshots {
    pub fn with_interval(values: Vec<(DateTime<Utc>, f64)>, interval: Interval) -> Self {
        let values = thin_by_interval(values, interval);
        let values = values
            .into_iter()
            .map(|(timestamp, value)| CoinAveragePriceHistoricalSnapshot { value, timestamp })
            .collect();

        Self(values)
    }
}

#[derive(Serialize, Clone)]
pub struct CoinAveragePriceHistoricalSnapshot {
    value: f64,
    #[serde(with = "ser_date_into_timestamp")]
    timestamp: DateTime<Utc>,
}
