use crate::worker::helper_functions::date_time_from_timestamp_sec;
use chrono::{DateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.timestamp() as u64;

    serializer.serialize_u64(s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp: u64 = Deserialize::deserialize(deserializer)?;

    Ok(date_time_from_timestamp_sec(timestamp))
}
