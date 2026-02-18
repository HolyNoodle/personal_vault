use axum::{extract::{State, Path}, http::StatusCode, response::IntoResponse, Json};
use crate::infrastructure::AppState;
use crate::application::invite::initiate_webauthn_registration;

pub async fn initiate_webauthn_registration(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    match initiate_webauthn_registration::execute(&state, &token).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "challenge_id": result.challenge_id,
                "challenge": result.challenge,
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
