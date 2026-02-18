use crate::domain::value_objects::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::invitation::AccessLevel;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilePermission {
    pub id: Uuid,
    pub owner_id: UserId,
    pub client_id: UserId,
    pub path: String,
    pub access: Vec<AccessLevel>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl FilePermission {
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at.map(|e| e > Utc::now()).unwrap_or(true)
    }
    pub fn allows(&self, level: AccessLevel) -> bool {
        self.access.contains(&level)
    }
    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }
}
