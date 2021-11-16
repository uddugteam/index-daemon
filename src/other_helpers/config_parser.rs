use std::collections::HashMap;

pub struct ConfigParser {
    path: String,
    params: HashMap<String, String>,
}

impl ConfigParser {
    // TODO: Implement
    pub fn new(path: &String) -> ConfigParser {
        ConfigParser {
            path: String::from(path),
            params: HashMap::new(),
        }
    }

    pub fn getParam(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|v| &v[..])
    }
}
