use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

pub struct ExchangePairInfoRepository {
    pub volume_24h: Arc<Mutex<dyn RepositoryKeyless<f64> + Send>>,
    pub timestamp: Arc<Mutex<dyn RepositoryKeyless<DateTime<Utc>> + Send>>,
}

pub trait Repository<T, U> {
    fn read(&self, primary: T) -> Option<U>;
    fn insert(&mut self, primary: T, new_value: U);
    fn delete(&mut self, primary: T);
}

pub trait RepositoryKeyless<U> {
    fn read(&self) -> Option<U>;
    fn insert(&mut self, new_value: U);
    fn delete(&mut self);
}
