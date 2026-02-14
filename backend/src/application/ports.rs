// Application ports (interfaces) - Define abstractions that infrastructure implements

use axum::async_trait;
use uuid::Uuid;
use webauthn_rs::prelude::Passkey;

// Domain types (will move to domain layer later)
pub type UserId = Uuid;

#[derive(Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub role: UserRole,
    pub status: UserStatus,
}

#[derive(Clone, Debug)]
pub enum UserRole {
    SuperAdmin,
    Owner,
    Client,
}

#[derive(Clone, Debug)]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

#[derive(Clone, Debug)]
pub struct WebAuthnCredential {
    pub user_id: UserId,
    pub credential_id: Vec<u8>,
    pub passkey: Passkey,
    pub sign_count: i64,
}

// Repository ports
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, String>;
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, String>;
    async fn create(&self, user: &User) -> Result<(), String>;
    async fn count_super_admins(&self) -> Result<i64, String>;
}

#[async_trait]
pub trait CredentialRepository: Send + Sync {
    async fn find_by_user_id(&self, user_id: &UserId) -> Result<Vec<WebAuthnCredential>, String>;
    async fn create(&self, credential: &WebAuthnCredential) -> Result<(), String>;
    async fn update_sign_count(&self, user_id: &UserId, credential_id: &[u8], sign_count: i64) -> Result<(), String>;
}

#[async_trait]
pub trait ChallengeRepository: Send + Sync {
    async fn save_registration_challenge(&self, challenge_id: &str, state: &str, ttl_seconds: u64) -> Result<(), String>;
    async fn get_and_delete_registration_challenge(&self, challenge_id: &str) -> Result<String, String>;
    async fn save_auth_challenge(&self, challenge_id: &str, state: &str, ttl_seconds: u64) -> Result<(), String>;
    async fn get_and_delete_auth_challenge(&self, challenge_id: &str) -> Result<String, String>;
}
