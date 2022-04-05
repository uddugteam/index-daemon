use chrono::{DateTime, Utc};
use serde::{self, Serializer};

pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.timestamp() as u64;

    serializer.serialize_u64(s)
}
