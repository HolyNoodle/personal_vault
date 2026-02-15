use axum::http::StatusCode;
use webauthn_rs::prelude::*;
use crate::infrastructure::AppState;

pub struct RegistrationResult {
    pub options: CreationChallengeResponse,
    pub challenge_id: String,
}

pub async fn execute(
    state: &AppState,
    email: &str,
    display_name: &str,
) -> Result<RegistrationResult, (StatusCode, String)> {
    let user_unique_id = uuid::Uuid::new_v4();
    
    // Generate WebAuthn challenge
    let (challenge, reg_state) = state.webauthn
        .start_passkey_registration(
            user_unique_id,
            email,
            display_name,
            None,
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let state_json = serde_json::to_string(&reg_state)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Store challenge using repository
    state.challenge_repo
        .save_registration_challenge(&challenge_id, &state_json, 300)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    Ok(RegistrationResult {
        options: challenge,
        challenge_id,
    })
}

#[cfg(test)]
mod tests {
    // Removed unused import: use super::*;
    
    // TODO: Add tests with mock repositories
    #[tokio::test]
    async fn test_initiate_registration_creates_challenge() {
        // Test implementation with mocks
    }
}
