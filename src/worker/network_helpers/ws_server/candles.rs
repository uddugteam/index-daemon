use crate::worker::helper_functions::date_time_from_timestamp_sec;
use crate::worker::network_helpers::ws_server::ser_date_into_timestamp;
use crate::worker::network_helpers::ws_server::ws_channel_request::Interval;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
pub struct Candles(Vec<Candle>);

impl Candles {
    pub fn calculate(values: Vec<(DateTime<Utc>, f64)>, interval: Interval) -> Self {
        let mut values: Vec<(i64, f64)> =
            values.into_iter().map(|v| (v.0.timestamp(), v.1)).collect();
        values.sort_by(|a, b| a.0.cmp(&b.0));

        let interval = interval.into_seconds() as i64;

        let mut last_to = values[0].0 + interval;
        let mut chunks = Vec::new();
        chunks.push(Vec::new());
        values.into_iter().for_each(|(t, v)| {
            if t < last_to {
                let t = date_time_from_timestamp_sec(t);
                chunks.last_mut().unwrap().push((t, v));
            } else {
                chunks.push(Vec::new());
                last_to += interval;
            }
        });
        let candles = chunks
            .into_iter()
            .filter(|v| !v.is_empty())
            .map(|v| {
                let t = v.last().unwrap().0;
                Candle::calculate(v, t).unwrap()
            })
            .collect();

        Self(candles)
    }
}

#[derive(Serialize, Clone)]
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
    pub fn calculate(
        mut values: Vec<(DateTime<Utc>, f64)>,
        timestamp: DateTime<Utc>,
    ) -> Option<Self> {
        values.sort_by(|a, b| a.0.cmp(&b.0));

        if !values.is_empty() {
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
