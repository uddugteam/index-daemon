use crate::repository::repository::RepositoryKeyless;

pub struct F64Repository(Option<f64>);

impl F64Repository {
    pub fn new() -> Self {
        Self(None)
    }
}

impl RepositoryKeyless<f64> for F64Repository {
    fn read(&self) -> Option<f64> {
        self.0
    }

    fn insert(&mut self, new_value: f64) {
        self.0 = Some(new_value);
    }

    fn delete(&mut self) {
        self.0 = None;
    }
}
