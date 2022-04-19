use crate::worker::helper_functions::date_time_from_timestamp_sec;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candles(Vec<Candle>);

impl Candles {
    pub fn calculate(values: Vec<(DateTime<Utc>, f64)>, interval_sec: u64) -> Self {
        let values: Vec<(u64, f64)> = values
            .into_iter()
            .map(|v| (v.0.timestamp() as u64, v.1))
            .collect();

        let candles = if !values.is_empty() {
            let mut last_to = values[0].0 + interval_sec;
            let mut chunks = Vec::new();
            chunks.push(Vec::new());
            values.into_iter().for_each(|(t, v)| {
                if t > last_to {
                    chunks.push(Vec::new());
                    last_to = t + interval_sec;
                }

                let t = date_time_from_timestamp_sec(t);
                chunks.last_mut().unwrap().push((t, v));
            });

            chunks
                .into_iter()
                .filter(|v| !v.is_empty())
                .map(|v| Candle::calculate(v).unwrap())
                .collect()
        } else {
            Vec::new()
        };

        Self(candles)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candle {
    open: f64,
    close: f64,
    min: f64,
    max: f64,
    avg: f64,
    #[serde(with = "ser_date_into_timestamp")]
    pub timestamp: DateTime<Utc>,
}

impl Candle {
    pub fn calculate(values: Vec<(DateTime<Utc>, f64)>) -> Option<Self> {
        if !values.is_empty() {
            let timestamp = values.last().unwrap().0;

            let open = values.first().unwrap().1;
            let close = values.last().unwrap().1;
            let mut min = values.first().unwrap().1;
            let mut max = values.first().unwrap().1;

            let mut sum = 0.0;
            let count = values.len();

            for (_, value) in values {
                if value < min {
                    min = value;
                }

                if value > max {
                    max = value;
                }

                sum += value;
            }

            let avg = sum / count as f64;

            Some(Self {
                open,
                close,
                min,
                max,
                avg,
                timestamp,
            })
        } else {
            None
        }
    }
}
