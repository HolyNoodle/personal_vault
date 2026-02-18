use webauthn_rs::prelude::*;
use jsonwebtoken::{encode, Header, EncodingKey};
use crate::infrastructure::AppState;
use crate::domain::{User, Credential, Email, DisplayName};
use crate::domain::value_objects::user_role::UserRole;
use crate::domain::entities::file_permission::FilePermission;

pub struct InviteCompleteResult {
    pub token: String,
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
}

pub async fn execute(
    state: &AppState,
    token: &str,
    challenge_id: &str,
    credential: RegisterPublicKeyCredential,
) -> Result<InviteCompleteResult, String> {
    // 1. Look up and validate invitation
    let invitation = state
        .invitation_repo
        .find_by_token(token)
        .await?
        .ok_or_else(|| "Invitation not found".to_string())?;

    if !invitation.is_valid() {
        return Err("Invitation is expired or revoked".to_string());
    }

    // 2. Get registration challenge and finish WebAuthn registration
    let state_json = state
        .challenge_repo
        .get_and_delete_registration_challenge(challenge_id)
        .await
        .map_err(|e| format!("Challenge not found or expired: {e}"))?;

    let reg_state: PasskeyRegistration = serde_json::from_str(&state_json)
        .map_err(|e| format!("Failed to deserialize registration state: {e}"))?;

    let passkey = state
        .webauthn
        .finish_passkey_registration(&credential, &reg_state)
        .map_err(|e| format!("WebAuthn registration failed: {e}"))?;

    // 3. Find or create user
    let email_str = invitation.invitee_email.as_str().to_string();
    let user_email = Email::new(email_str.clone())
        .map_err(|e| format!("Invalid email: {e}"))?;

    let user = match state.user_repo.find_by_email(&user_email).await? {
        Some(existing) => existing,
        None => {
            let display_name = DisplayName::new(email_str.clone())
                .map_err(|e| format!("Invalid display name: {e}"))?;
            let new_user = User::new(user_email, display_name, vec![UserRole::Client]);
            state.user_repo.save(&new_user).await?;
            new_user
        }
    };

    // 4. Save credential
    let cred = Credential::new(user.id().clone(), passkey);
    state.credential_repo.save(&cred).await?;

    // 5. Create FilePermission rows for each granted path
    for granted_path in &invitation.granted_paths {
        let permission = FilePermission {
            id: uuid::Uuid::new_v4(),
            owner_id: invitation.owner_id.clone(),
            client_id: user.id().clone(),
            path: granted_path.path.clone(),
            access: granted_path.access.clone(),
            granted_at: chrono::Utc::now(),
            expires_at: invitation.expires_at,
            revoked_at: None,
        };
        state.file_permission_repo.save(&permission).await?;
    }

    // 6. Mark invitation as accepted
    state
        .invitation_repo
        .update_status(&invitation.id, "Accepted")
        .await?;

    // 7. Generate JWT
    #[derive(serde::Serialize)]
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

    let role_strings: Vec<String> = user
        .roles()
        .iter()
        .map(|r| r.as_db_str().to_string())
        .collect();

    let claims = Claims {
        sub: user.id().to_string(),
        email: email_str.clone(),
        roles: role_strings.clone(),
        exp: expiration,
    };

    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to generate JWT: {e}"))?;

    Ok(InviteCompleteResult {
        token: jwt,
        user_id: user.id().to_string(),
        email: email_str,
        roles: role_strings,
    })
}
