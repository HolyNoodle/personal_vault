use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        }
    }
    
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "active" => Ok(UserStatus::Active),
            "suspended" => Ok(UserStatus::Suspended),
            "deleted" => Ok(UserStatus::Deleted),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}
