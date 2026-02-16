use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    SuperAdmin,
    Owner,
    Client,
}

impl UserRole {
    // Removed unused methods as_str and from_str
    pub fn as_db_str(&self) -> &'static str {
        match self {
            UserRole::SuperAdmin => "super_admin",
            UserRole::Owner => "owner",
            UserRole::Client => "client",
        }
    }
}
