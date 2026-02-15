// Infrastructure layer - external concerns (database, filesystem, etc.)
// Implements interfaces defined in application layer

use std::sync::Arc;
// ...existing code...
use crate::application::ports::{CredentialRepository, ChallengeRepository};

pub mod driven;    // Output adapters (repositories, external services)
pub mod driving;   // Input adapters (HTTP, CLI, etc.)

#[derive(Clone)]
pub struct AppState {
    pub webauthn: Arc<webauthn_rs::prelude::Webauthn>,
    pub jwt_secret: String,
    // Removed orphaned user_repo field
    pub credential_repo: Arc<dyn CredentialRepository>,
    pub challenge_repo: Arc<dyn ChallengeRepository>,
}

// Removed unused structs AuthUser and Claims

// Removed unused AuthUser and Claims extraction logic
