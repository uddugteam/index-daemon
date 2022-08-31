use chrono::{DateTime, NaiveDateTime, Utc};

pub fn get_pair_ref(pair: &(String, String)) -> (&str, &str) {
    (pair.0.as_str(), pair.1.as_str())
}

pub fn strip_usd(pair: &(String, String)) -> Option<String> {
    match get_pair_ref(pair) {
        ("USD", coin) | (coin, "USD") => {
            // good pair (coin-USD)
            Some(coin.to_string())
        }
        _ => {
            // bad pair (coin-coin)
            None
        }
    }
}

pub fn date_time_subtract_sec(minuend: DateTime<Utc>, subtrahend_sec: u64) -> DateTime<Utc> {
    let minuend = minuend.timestamp() as u64;

    date_time_from_timestamp_sec(minuend - subtrahend_sec)
}

pub fn min_date_time() -> DateTime<Utc> {
    date_time_from_timestamp_sec(0)
}

pub fn date_time_from_timestamp_sec(timestamp_sec: u64) -> DateTime<Utc> {
    let naive = NaiveDateTime::from_timestamp(timestamp_sec as i64, 0);

    DateTime::from_utc(naive, Utc)
}

pub fn date_time_from_timestamp_millis(timestamp_millis: u64) -> DateTime<Utc> {
    date_time_from_timestamp_sec(timestamp_millis / 1000)
}
