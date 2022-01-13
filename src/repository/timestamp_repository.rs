use crate::repository::repository::RepositoryKeyless;
use chrono::{DateTime, Utc};

pub struct TimestampRepository(Option<DateTime<Utc>>);

impl TimestampRepository {
    pub fn new() -> Self {
        Self(None)
    }
}

impl RepositoryKeyless<DateTime<Utc>> for TimestampRepository {
    fn read(&self) -> Option<DateTime<Utc>> {
        self.0
    }

    fn insert(&mut self, new_value: DateTime<Utc>) {
        self.0 = Some(new_value);
    }

    fn delete(&mut self) {
        self.0 = None;
    }
}
