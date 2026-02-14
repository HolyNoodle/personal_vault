// Infrastructure layer - external concerns (database, filesystem, etc.)
// Implements interfaces defined in application layer

use std::sync::Arc;
use axum::{async_trait, extract::{FromRequestParts, FromRef}, http::{StatusCode, request::Parts}};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use crate::application::ports::{UserRepository, CredentialRepository, ChallengeRepository};

pub mod driven;    // Output adapters (repositories, external services)
pub mod driving;   // Input adapters (HTTP, CLI, etc.)

#[derive(Clone)]
pub struct AppState {
    pub webauthn: Arc<webauthn_rs::prelude::Webauthn>,
    pub jwt_secret: String,
    pub user_repo: Arc<dyn UserRepository>,
    pub credential_repo: Arc<dyn CredentialRepository>,
    pub challenge_repo: Arc<dyn ChallengeRepository>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: usize,
}

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub role: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing authorization header".to_string()))?;
        
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid authorization format".to_string()))?;
        
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(app_state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;
        
        let user_id = uuid::Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid user ID".to_string()))?;
        
        Ok(AuthUser {
            user_id,
            email: token_data.claims.email,
            role: token_data.claims.role,
        })
    }
}
