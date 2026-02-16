use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl UserStatus {
    // Removed unused methods as_str and from_str
    pub fn as_db_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        }
    }
}
