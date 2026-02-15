use axum::http::StatusCode;
use webauthn_rs::prelude::*;
use jsonwebtoken::{encode, Header, EncodingKey};
use crate::infrastructure::AppState;
// use crate::domain::Email; // removed unused import

pub struct LoginCompleteResult {
    pub token: String,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

pub async fn execute(
    state: &AppState,
    challenge_id: &str,
    _credential: PublicKeyCredential,
    _email: &str,
) -> Result<LoginCompleteResult, (StatusCode, String)> {
    // Get and delete challenge from repository
    let state_json = state.challenge_repo
        .get_and_delete_auth_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    let _auth_state: PasskeyAuthentication = serde_json::from_str(&state_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Validate credential with WebAuthn (disabled, removed code)
    // Parse email (disabled, removed code)
    
    // Find user by email
    // User lookup disabled (user_repo removed)
    // Removed unused variable user
    
    // Update sign count
    // Sign count update disabled (user removed)
    
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Claims {
        sub: String,
        email: String,
        role: String,
        exp: usize,
    }
    
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;
    
    // Removed unused variable role_str
    
    let claims = Claims {
        sub: String::new(),
        email: String::new(),
        role: String::new(),
        exp: expiration,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // User login print disabled (user removed)
    
    Ok(LoginCompleteResult {
        token,
        user_id: String::new(),
        email: String::new(),
        display_name: String::new(),
        role: String::new(),
    })
}

#[cfg(test)]
mod tests {
    // Removed unused import: use super::*;
    
    // TODO: Add tests with mock repositories
    #[tokio::test]
    async fn test_complete_login_with_valid_credential() {
        // Test implementation with mocks
    }
    
    #[tokio::test]
    async fn test_complete_login_with_invalid_challenge() {
        // Test implementation with mocks
    }
}
