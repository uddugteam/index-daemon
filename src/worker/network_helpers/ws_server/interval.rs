#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

impl Interval {
    pub fn into_seconds(self) -> u64 {
        match self {
            Self::Second => 1,
            Self::Minute => 60,
            Self::Hour => 3600,
            Self::Day => 86400,
            Self::Week => 604800,
            Self::Month => 2592000,
        }
    }
}
