use axum::http::StatusCode;
use webauthn_rs::prelude::*;
use jsonwebtoken::{encode, Header, EncodingKey};
use crate::infrastructure::AppState;

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
    credential: PublicKeyCredential,
    email: &str,
) -> Result<LoginCompleteResult, (StatusCode, String)> {
    // Get and delete challenge from repository
    let state_json = state.challenge_repo
        .get_and_delete_auth_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    let auth_state: PasskeyAuthentication = serde_json::from_str(&state_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Validate credential with WebAuthn
    let auth_result = state.webauthn
        .finish_passkey_authentication(&credential, &auth_state)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    
    // Find user by email
    let user = state.user_repo.find_by_email(email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;
    
    // Update sign count
    state.credential_repo
        .update_sign_count(&user.id, auth_result.cred_id().0.as_slice(), auth_result.counter() as i64)
        .await
        .ok(); // Non-critical, don't fail if update fails
    
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
    
    let role_str = format!("{:?}", user.role);
    
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: role_str.clone(),
        exp: expiration,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    println!("User logged in: {} ({})", user.email, user.id);
    
    Ok(LoginCompleteResult {
        token,
        user_id: user.id.to_string(),
        email: user.email,
        display_name: user.display_name,
        role: role_str,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
