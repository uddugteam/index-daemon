use crate::worker::network_helpers::ws_server::hepler_functions::thin_by_interval;
use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
pub struct F64Snapshots(Vec<F64Snapshot>);

impl F64Snapshots {
    pub fn with_interval(values: Vec<(DateTime<Utc>, f64)>, interval: Interval) -> Self {
        let values = thin_by_interval(values, interval);
        let values = values
            .into_iter()
            .map(|(timestamp, value)| F64Snapshot { value, timestamp })
            .collect();

        Self(values)
    }
}

#[derive(Serialize, Clone)]
pub struct F64Snapshot {
    value: f64,
    #[serde(with = "ser_date_into_timestamp")]
    timestamp: DateTime<Utc>,
}
