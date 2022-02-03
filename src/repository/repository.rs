pub trait Repository<T, U> {
    fn read(&self, primary: T) -> Result<Option<U>, String>;
    fn insert(&self, primary: T, new_value: U) -> Result<(), String>;
    fn delete(&self, primary: T);
}

pub trait RepositoryKeyless<U: Clone> {
    fn read(&self) -> U;
    fn insert(&self, new_value: U) -> Result<(), String>;
}
