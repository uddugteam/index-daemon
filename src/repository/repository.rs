use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

pub struct ExchangePairInfoRepository {
    pub volume_24h: Arc<Mutex<dyn CrudKeyless<f64> + Send>>,
    pub timestamp: Arc<Mutex<dyn CrudKeyless<DateTime<Utc>> + Send>>,
}

pub trait Crud<T, U> {
    fn read(&self, primary: T) -> Option<U>;
    fn insert(&mut self, primary: T, new_value: U);
    fn delete(&mut self, primary: T);
}

pub trait CrudKeyless<U> {
    fn read(&self) -> Option<U>;
    fn insert(&mut self, new_value: U);
    fn delete(&mut self);
}
