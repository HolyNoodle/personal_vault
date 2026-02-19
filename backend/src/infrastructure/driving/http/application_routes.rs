use axum::{extract::State, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use crate::infrastructure::AppState;
use crate::infrastructure::driving::http::middleware::auth::AuthenticatedUser;
use crate::application::client::commands::launch_application;

#[derive(Serialize)]
pub struct ApplicationMetadata {
    pub app_id: String,
    pub name: String,
    pub description: String,
}

/// Returns a list of available applications
pub async fn list_applications() -> Json<Vec<ApplicationMetadata>> {
    let apps = vec![
        ApplicationMetadata {
            app_id: "file_explorer".to_string(),
            name: "File Explorer".to_string(),
            description: "Browse and manage files in your sandboxed environment.".to_string(),
        },
    ];
    Json(apps)
}

#[derive(Deserialize)]
pub struct LaunchApplicationRequest {
    pub app_id: String,
    #[serde(default = "default_width")]
    pub width: u16,
    #[serde(default = "default_height")]
    pub height: u16,
}

fn default_width() -> u16 { 1280 }
fn default_height() -> u16 { 720 }

#[derive(Serialize)]
pub struct LaunchApplicationResponse {
    pub session_id: String,
    pub websocket_url: String,
}

pub async fn launch_application(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<LaunchApplicationRequest>,
) -> impl IntoResponse {
    match launch_application::execute(&state, &user, &payload.app_id, payload.width, payload.height).await {
        Ok(result) => (
            StatusCode::OK,
            Json(LaunchApplicationResponse {
                session_id: result.session_id,
                websocket_url: result.websocket_url,
            }),
        ).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
