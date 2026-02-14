use axum::http::StatusCode;
use webauthn_rs::prelude::*;
use crate::infrastructure::AppState;
use crate::application::ports::{User, UserRole, UserStatus, WebAuthnCredential};

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
    let user_id = uuid::Uuid::new_v4();
    let user = User {
        id: user_id,
        email: email.to_string(),
        display_name: display_name.to_string(),
        role: UserRole::SuperAdmin,
        status: UserStatus::Active,
    };
    
    println!("Creating user with id: {}", user_id);
    
    // Persist user through repository
    state.user_repo.create(&user)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    println!("User created, now creating credential");
    
    // Create credential entity
    let credential = WebAuthnCredential {
        user_id,
        credential_id: passkey.cred_id().0.to_vec(),
        passkey,
        sign_count: 0,
    };
    
    // Persist credential through repository
    state.credential_repo.create(&credential)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    println!("Super admin created: {} ({})", email, user_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: Add tests with mock repositories
    #[tokio::test]
    async fn test_complete_registration_creates_user_and_credential() {
        // Test implementation with mocks
    }
}
