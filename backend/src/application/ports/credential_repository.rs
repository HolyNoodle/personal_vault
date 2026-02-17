// Driven port - Credential repository (output port)

use axum::async_trait;
use crate::domain::{Credential, UserId};

#[async_trait]
pub trait CredentialRepository: Send + Sync {
    async fn find_by_user_id(&self, user_id: &UserId) -> Result<Vec<Credential>, String>;
    async fn save(&self, credential: &Credential) -> Result<(), String>;
}
