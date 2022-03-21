use crate::worker::network_helpers::ws_server::interval::Interval;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use chrono::{DateTime, Utc, MIN_DATETIME};

#[derive(Debug, Serialize, Clone)]
pub struct F64Snapshots(Vec<F64Snapshot>);

impl F64Snapshots {
    pub fn with_interval(values: Vec<(DateTime<Utc>, f64)>, interval: Interval) -> Self {
        let values = Self::thin_by_interval(values, interval);
        let values = values
            .into_iter()
            .map(|(timestamp, value)| F64Snapshot { value, timestamp })
            .collect();

        Self(values)
    }

    fn thin_by_interval(
        values: Vec<(DateTime<Utc>, f64)>,
        interval: Interval,
    ) -> Vec<(DateTime<Utc>, f64)> {
        let interval = interval.into_seconds() as i64;

        let mut res = Vec::new();

        let mut next_timestamp = MIN_DATETIME.timestamp();
        for value in values {
            let curr_timestamp = value.0.timestamp();

            if curr_timestamp >= next_timestamp {
                res.push(value);
                next_timestamp = curr_timestamp + interval;
            }
        }

        res
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct F64Snapshot {
    value: f64,
    #[serde(with = "ser_date_into_timestamp")]
    timestamp: DateTime<Utc>,
}
