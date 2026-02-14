use std::collections::HashMap;

const VERSION: &str = "1.0";

/// A documented struct
#[derive(Debug, Clone)]
struct Config {
    name: String,
    values: HashMap<String, i32>,
}

/// Creates a new config
pub fn create_config(name: &str) -> Config {
    let mut values = HashMap::new();
    values.insert("default".to_string(), 42);
    Config {
        name: name.to_string(),
        values,
    }
}

impl Config {
    /// Gets a value
    pub fn get(&self, key: &str) -> Option<&i32> {
        self.values.get(key)
    }

    fn internal_helper(&self) -> bool {
        self.values.len() > 0 && self.name.len() > 0
    }
}

trait Validator {
    fn validate(&self) -> bool {
        true
    }
}
