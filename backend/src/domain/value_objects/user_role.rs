use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    SuperAdmin,
    Owner,
    Client,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::SuperAdmin => "super_admin",
            UserRole::Owner => "owner",
            UserRole::Client => "client",
        }
    }
    
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "super_admin" => Ok(UserRole::SuperAdmin),
            "owner" => Ok(UserRole::Owner),
            "client" => Ok(UserRole::Client),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}
