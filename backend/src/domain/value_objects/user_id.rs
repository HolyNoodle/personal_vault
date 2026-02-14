use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
    
    pub fn from_uuid(id: uuid::Uuid) -> Self {
        Self(id)
    }
    
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
