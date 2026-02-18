use axum::{extract::{State, Path}, http::StatusCode, response::IntoResponse, Json};
use crate::infrastructure::AppState;
use crate::application::invite::view_invitation;

pub async fn view_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    match view_invitation::execute(&*state.invitation_repo, &token).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(e) if e.contains("not found") => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
