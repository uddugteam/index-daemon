use std::collections::HashMap;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};

pub struct ConfigParser {
    params: HashMap<String, String>,
}

impl ConfigParser {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        let mut params: HashMap<String, String> = HashMap::new();

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for (line_number, line) in reader.lines().enumerate() {
            let line = line?;

            if let Some(line) = line.split('#').collect::<Vec<&str>>().first() {
                if !line.is_empty() {
                    let parts = line.split('=').collect::<Vec<&str>>();
                    if !parts.is_empty() {
                        let param_code = *parts.first().unwrap_or_else(|| {
                            panic!("Param code read error, line number: {}", line_number);
                        });
                        if param_code.is_empty() {
                            panic!("Param code is empty, line number: {}", line_number);
                        }
                        let param_code = if param_code.contains(' ') {
                            panic!("Param code contains space");
                        } else {
                            param_code
                        };
                        let param_value = *parts
                            .get(1)
                            .unwrap_or_else(|| {
                                panic!("Param value read error, line number: {}", line_number);
                            })
                            .split(';')
                            .collect::<Vec<&str>>()
                            .first()
                            .unwrap_or_else(|| {
                                panic!("Param value read error, line number: {}", line_number);
                            });

                        params.insert(param_code.to_string(), param_value.to_string());
                    }
                }
            }
        }

        Ok(ConfigParser { params })
    }

    pub fn getParam(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|v| &v[..])
    }
}
