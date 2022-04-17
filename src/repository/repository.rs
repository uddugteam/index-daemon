use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dyn_clone::{clone_trait_object, DynClone};

#[async_trait]
pub trait Repository<T, U>: DynClone {
    async fn read(&self, primary: T) -> Result<Option<U>, String>;
    async fn read_range(&self, primary_from: T, primary_to: T) -> Result<Vec<(T, U)>, String>;
    async fn insert(&mut self, primary: T, new_value: U) -> Option<Result<(), String>>;
    async fn delete(&mut self, primary: T);
    async fn delete_multiple(&mut self, primary: &[T]);
}

clone_trait_object!(Repository<DateTime<Utc>, f64>);
