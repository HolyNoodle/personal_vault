use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(email: String) -> Result<Self, String> {
        if !email.contains('@') {
            return Err("Invalid email format".to_string());
        }
        if email.len() > 255 {
            return Err("Email too long".to_string());
        }
        Ok(Self(email.to_lowercase()))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
