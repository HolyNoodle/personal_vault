use axum::http::StatusCode;
use webauthn_rs::prelude::*;
use crate::infrastructure::AppState;
use crate::domain::Email;

pub struct LoginInitiateResult {
    pub options: RequestChallengeResponse,
    pub challenge_id: String,
}

pub async fn execute(
    state: &AppState,
    email: &str,
) -> Result<LoginInitiateResult, (StatusCode, String)> {
    // Parse email
    let email = Email::new(email.to_string())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    // Find user by email
    let user = state.user_repo.find_by_email(&email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;
    
    // Get user's credentials
    let credentials = state.credential_repo.find_by_user_id(user.id())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    println!("Found {} credentials for user {}", credentials.len(), email);
    
    if credentials.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No credentials found for user".to_string()));
    }
    
    let passkeys: Vec<Passkey> = credentials
        .into_iter()
        .map(|c| c.passkey().clone())
        .collect();
    
    // Generate WebAuthn challenge
    let (challenge, auth_state) = state.webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let state_json = serde_json::to_string(&auth_state)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Store challenge using repository
    state.challenge_repo
        .save_auth_challenge(&challenge_id, &state_json, 300)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    Ok(LoginInitiateResult {
        options: challenge,
        challenge_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: Add tests with mock repositories
    #[tokio::test]
    async fn test_initiate_login_with_valid_user() {
        // Test implementation with mocks
    }
    
    #[tokio::test]
    async fn test_initiate_login_with_no_credentials() {
        // Test implementation with mocks
    }
}
