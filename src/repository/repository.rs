use chrono::{DateTime, Utc};
use dyn_clone::{clone_trait_object, DynClone};
use std::collections::HashMap;

pub trait Repository<T, U>: DynClone {
    fn read(&self, primary: T) -> Result<Option<U>, String>;
    fn read_range(&self, primary_from: T, primary_to: T) -> Result<HashMap<T, U>, String>;
    fn insert(&mut self, primary: T, new_value: U) -> Option<Result<(), String>>;
    fn delete(&mut self, primary: T);
}

clone_trait_object!(Repository<DateTime<Utc>, f64>);

pub trait RepositoryKeyless<U: Clone> {
    fn read(&self) -> U;
    fn insert(&mut self, new_value: U) -> Option<Result<(), String>>;
}
