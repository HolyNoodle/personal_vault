// Infrastructure layer - external concerns (database, filesystem, etc.)
// Implements interfaces defined in application layer

use std::sync::Arc;
use crate::application::ports::{CredentialRepository, ChallengeRepository, InvitationRepository, FilePermissionRepository};

pub mod driven;    // Output adapters (repositories, external services)
pub mod driving;   // Input adapters (HTTP, CLI, etc.)

#[derive(Clone)]
pub struct AppState {
    pub webauthn: Arc<webauthn_rs::prelude::Webauthn>,
    pub jwt_secret: String,
    pub user_repo: Arc<dyn crate::application::ports::user_repository::UserRepository>,
    pub credential_repo: Arc<dyn CredentialRepository>,
    pub challenge_repo: Arc<dyn ChallengeRepository>,
    pub invitation_repo: Arc<dyn InvitationRepository>,
    pub file_permission_repo: Arc<dyn FilePermissionRepository>,
}
