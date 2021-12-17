pub trait Repository<T, U> {
    fn read(&self, primary: T) -> Option<U>;
    fn insert(&mut self, primary: T, new_value: U);
    fn delete(&mut self, primary: T);
}
