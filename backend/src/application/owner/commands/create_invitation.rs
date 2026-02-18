use crate::domain::entities::invitation::{Invitation, InvitationStatus, GrantedPath, AccessLevel};
use crate::domain::value_objects::{Email, UserId};
use crate::application::ports::invitation_repository::InvitationRepository;
use chrono::{Utc, Duration};
use uuid::Uuid;

pub struct CreateInvitationCommand {
    pub owner_id: UserId,
    pub invitee_email: String,
    pub granted_paths: Vec<GrantedPath>,
    pub expires_in_hours: Option<i64>,
}

pub struct CreateInvitationResult {
    pub invitation_id: Uuid,
    pub token: String,
    pub invite_url: String,
}

pub async fn execute<R: InvitationRepository + ?Sized>(
    repo: &R,
    cmd: CreateInvitationCommand,
    base_url: &str,
) -> Result<CreateInvitationResult, String> {
    // Validate email
    let email = Email::new(cmd.invitee_email).map_err(|e| e.to_string())?;
    // Validate paths (relative, no ..)
    for gp in &cmd.granted_paths {
        if gp.path.contains("..") || gp.path.starts_with('/') {
            return Err("Invalid path: must be a relative path without '..'".to_string());
        }
    }
    let token = uuid::Uuid::new_v4().simple().to_string();
    let expires_at = cmd.expires_in_hours.map(|h| Utc::now() + Duration::hours(h));
    let invitation = Invitation {
        id: Uuid::new_v4(),
        owner_id: cmd.owner_id,
        invitee_email: email,
        token: token.clone(),
        granted_paths: cmd.granted_paths,
        status: InvitationStatus::Pending,
        expires_at,
        created_at: Utc::now(),
    };
    repo.save(&invitation).await?;
    let invite_url = format!("{}/invite/{}", base_url.trim_end_matches('/'), token);
    Ok(CreateInvitationResult {
        invitation_id: invitation.id,
        token,
        invite_url,
    })
}
