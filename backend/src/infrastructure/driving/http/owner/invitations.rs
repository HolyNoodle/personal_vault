use axum::{extract::{State, Json}, http::StatusCode, response::IntoResponse};
use crate::infrastructure::AppState;
use crate::application::owner::commands::create_invitation::{self, CreateInvitationCommand};
use crate::infrastructure::driving::http::middleware::auth::AuthenticatedUser;

#[derive(serde::Deserialize)]
pub struct InvitationRequest {
    pub invitee_email: String,
    pub granted_paths: Vec<crate::domain::entities::invitation::GrantedPath>,
    pub expires_in_hours: Option<i64>,
}

pub async fn create_invitation(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<InvitationRequest>,
) -> impl IntoResponse {
    if !user.roles.contains(&crate::domain::value_objects::user_role::UserRole::Owner) {
        return (StatusCode::FORBIDDEN, "Not an owner").into_response();
    }
    let cmd = CreateInvitationCommand {
        owner_id: user.id.clone(),
        invitee_email: req.invitee_email,
        granted_paths: req.granted_paths,
        expires_in_hours: req.expires_in_hours,
    };
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    match create_invitation::execute(&*state.invitation_repo, cmd, &base_url).await {
        Ok(res) => (StatusCode::OK, Json(serde_json::json!({
            "invitation_id": res.invitation_id,
            "token": res.token,
            "invite_url": res.invite_url,
        }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
