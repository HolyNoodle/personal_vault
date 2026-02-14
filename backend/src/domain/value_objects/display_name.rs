use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn new(name: String) -> Result<Self, String> {
        if name.is_empty() {
            return Err("Display name cannot be empty".to_string());
        }
        if name.len() > 255 {
            return Err("Display name too long".to_string());
        }
        Ok(Self(name))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
