use webauthn_rs::prelude::*;
use crate::infrastructure::AppState;

pub struct InitiateInviteResult {
    pub challenge_id: String,
    pub challenge: CreationChallengeResponse,
}

pub async fn execute(state: &AppState, token: &str) -> Result<InitiateInviteResult, String> {
    let invitation = state
        .invitation_repo
        .find_by_token(token)
        .await?
        .ok_or_else(|| "Invitation not found".to_string())?;

    if !invitation.is_valid() {
        return Err("Invitation is expired or revoked".to_string());
    }

    let user_unique_id = uuid::Uuid::new_v4();
    let email = invitation.invitee_email.as_str();

    let (challenge, reg_state) = state
        .webauthn
        .start_passkey_registration(user_unique_id, email, email, None)
        .map_err(|e| format!("Failed to start passkey registration: {e}"))?;

    let challenge_id = uuid::Uuid::new_v4().to_string();
    let state_json = serde_json::to_string(&reg_state)
        .map_err(|e| format!("Failed to serialize registration state: {e}"))?;

    state
        .challenge_repo
        .save_registration_challenge(&challenge_id, &state_json, 300)
        .await
        .map_err(|e| format!("Failed to save challenge: {e}"))?;

    Ok(InitiateInviteResult {
        challenge_id,
        challenge,
    })
}
