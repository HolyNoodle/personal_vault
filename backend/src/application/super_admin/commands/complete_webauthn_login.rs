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
    pub roles: Vec<String>,
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

    // Find user by email
    let user_email = crate::domain::value_objects::Email::new(email.to_string())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user = state.user_repo
        .find_by_email(&user_email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

    // Find credentials for user
    let credentials = state.credential_repo
        .find_by_user_id(user.id())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if credentials.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "No credentials found for user".to_string()));
    }

    // Validate credential with WebAuthn
    let result = state.webauthn
        .finish_passkey_authentication(&credential, &auth_state)
        .map_err(|e| (StatusCode::FORBIDDEN, format!("WebAuthn verification failed: {e}")))?;

    // Update sign count using webauthn_rs passkey update
    let mut updated_passkey = credentials[0].passkey().clone();
    let _ = updated_passkey.update_credential(&result);
    let updated_cred = crate::domain::Credential::from_persistence(
        credentials[0].user_id().clone(),
        credentials[0].credential_id().to_vec(),
        updated_passkey,
        result.counter(),
    );
    state.credential_repo.save(&updated_cred)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Claims {
        sub: String,
        email: String,
        roles: Vec<String>,
        exp: usize,
    }

    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id().to_string(),
        email: user.email().as_str().to_string(),
        roles: user.roles().iter().map(|r| r.as_db_str().to_string()).collect(),
        exp: expiration,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(LoginCompleteResult {
        token,
        user_id: user.id().to_string(),
        email: user.email().as_str().to_string(),
        display_name: user.display_name().as_str().to_string(),
        roles: user.roles().iter().map(|r| r.as_db_str().to_string()).collect(),
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
