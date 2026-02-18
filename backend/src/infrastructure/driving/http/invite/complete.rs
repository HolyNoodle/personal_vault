use axum::{extract::{State, Path}, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use webauthn_rs::prelude::RegisterPublicKeyCredential;
use crate::infrastructure::AppState;
use crate::application::invite::complete_webauthn_registration;

#[derive(Deserialize)]
pub struct CompleteInviteRequest {
    pub challenge_id: String,
    pub credential: RegisterPublicKeyCredential,
}

pub async fn complete_webauthn_registration(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(req): Json<CompleteInviteRequest>,
) -> impl IntoResponse {
    match complete_webauthn_registration::execute(&state, &token, &req.challenge_id, req.credential).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "token": result.token,
                "user_id": result.user_id,
                "email": result.email,
                "roles": result.roles,
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
