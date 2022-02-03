use chrono::{DateTime, Utc};
use serde::{self, Serializer};

pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.timestamp();
    serializer.serialize_i64(s)
}
