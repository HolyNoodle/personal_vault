use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    SuperAdmin,
    Owner,
    Client,
}

impl UserRole {
    // Removed unused methods as_str and from_str
}
