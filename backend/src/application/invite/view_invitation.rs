use crate::application::ports::invitation_repository::InvitationRepository;
use crate::domain::entities::invitation::GrantedPath;

#[derive(Debug, serde::Serialize)]
pub struct InvitationView {
    pub owner_id: String,
    pub granted_paths: Vec<GrantedPath>,
    pub expires_at: Option<String>,
}

pub async fn execute(
    invitation_repo: &dyn InvitationRepository,
    token: &str,
) -> Result<InvitationView, String> {
    let invitation = invitation_repo
        .find_by_token(token)
        .await?
        .ok_or_else(|| "Invitation not found".to_string())?;

    if !invitation.is_valid() {
        return Err("Invitation is expired or revoked".to_string());
    }

    Ok(InvitationView {
        owner_id: invitation.owner_id.to_string(),
        granted_paths: invitation.granted_paths,
        expires_at: invitation.expires_at.map(|dt| dt.to_rfc3339()),
    })
}
