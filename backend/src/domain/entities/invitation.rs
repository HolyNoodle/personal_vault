use crate::domain::value_objects::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Invitation {
    pub id: Uuid,
    pub owner_id: UserId,
    pub invitee_email: Email,
    pub token: String,
    pub granted_paths: Vec<GrantedPath>,
    pub status: InvitationStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrantedPath {
    pub path: String,
    pub access: Vec<AccessLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AccessLevel {
    Read,
    Write,
    Delete,
}

impl Invitation {
    pub fn is_valid(&self) -> bool {
        self.status == InvitationStatus::Pending &&
            self.expires_at.map(|e| e > Utc::now()).unwrap_or(true)
    }
    pub fn mark_accepted(&mut self) {
        self.status = InvitationStatus::Accepted;
    }
    pub fn revoke(&mut self) {
        self.status = InvitationStatus::Revoked;
    }
}
