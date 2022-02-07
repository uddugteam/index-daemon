pub trait Repository<T, U> {
    fn read(&self, primary: T) -> Result<Option<U>, String>;
    fn insert(&mut self, primary: T, new_value: U) -> Option<Result<(), String>>;
    fn delete(&mut self, primary: T);
}

pub trait RepositoryKeyless<U: Clone> {
    fn read(&self) -> U;
    fn insert(&mut self, new_value: U) -> Option<Result<(), String>>;
}
