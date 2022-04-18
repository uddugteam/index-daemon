use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct F64Snapshots(Vec<F64Snapshot>);

impl F64Snapshots {
    pub fn with_interval(values: Vec<(DateTime<Utc>, f64)>, interval_sec: u64) -> Self {
        let values = Self::thin_by_interval(values, interval_sec);
        let values = values
            .into_iter()
            .map(|(timestamp, value)| F64Snapshot { value, timestamp })
            .collect();

        Self(values)
    }

    fn thin_by_interval(
        values: Vec<(DateTime<Utc>, f64)>,
        interval_sec: u64,
    ) -> Vec<(DateTime<Utc>, f64)> {
        let mut res = Vec::new();

        let mut next_timestamp = 0;
        for value in values {
            let curr_timestamp = value.0.timestamp() as u64;

            if curr_timestamp >= next_timestamp {
                res.push(value);
                next_timestamp = curr_timestamp + interval_sec;
            }
        }

        res
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct F64Snapshot {
    value: f64,
    #[serde(with = "ser_date_into_timestamp")]
    timestamp: DateTime<Utc>,
}
