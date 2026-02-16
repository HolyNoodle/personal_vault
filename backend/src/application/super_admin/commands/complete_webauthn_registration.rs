use axum::http::StatusCode;
use webauthn_rs::prelude::*;
use crate::infrastructure::AppState;
use crate::domain::{User, Credential, Email, DisplayName, UserRole};

pub async fn execute(
    state: &AppState,
    challenge_id: &str,
    credential: RegisterPublicKeyCredential,
    email: &str,
    display_name: &str,
) -> Result<(), (StatusCode, String)> {
    // Get challenge from repository
    let state_json = state.challenge_repo
        .get_and_delete_registration_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    let reg_state: PasskeyRegistration = serde_json::from_str(&state_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Validate credential with WebAuthn
    let passkey = state.webauthn
        .finish_passkey_registration(&credential, &reg_state)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    
    // Create user entity
    let user_email = Email::new(email.to_string())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_display_name = DisplayName::new(display_name.to_string())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    let user = User::new(user_email, user_display_name, UserRole::SuperAdmin);
    
    println!("Creating user with id: {}", user.id());
    
    // Persist user before credential
    state.user_repo.save(&user)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    println!("User created, now creating credential");
    
    // Create credential entity
    let credential = Credential::new(user.id().clone(), passkey);
    
    // Persist credential through repository
    state.credential_repo.save(&credential)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    println!("Super admin created: {} ({})", user.email(), user.id());
    Ok(())
}

#[cfg(test)]
mod tests {
    // Removed unused import: use super::*;
    
    // TODO: Add tests with mock repositories
    #[tokio::test]
    async fn test_complete_registration_creates_user_and_credential() {
        // Test implementation with mocks
    }
}
