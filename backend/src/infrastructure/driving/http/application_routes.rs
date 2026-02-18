use axum::debug_handler;
use axum::Json;
use serde::{Deserialize, Serialize};
// use crate::infrastructure::driving::http::video_api::ApiState;

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
    pub user_id: String,
}

#[derive(Serialize)]
pub struct LaunchApplicationResponse {
    pub session_id: String,
    pub websocket_url: String,
}

#[debug_handler]
pub async fn launch_application(
    Json(payload): Json<LaunchApplicationRequest>,
) -> Json<LaunchApplicationResponse> {
    tracing::info!(
        "[API] launch_application: app_id={}, user_id={}",
        payload.app_id,
        payload.user_id
    );

    let session_id = uuid::Uuid::new_v4().to_string();

    // The app is launched when WebRTC connects (on request-offer).
    // This endpoint just returns the session info for the client to connect.

    Json(LaunchApplicationResponse {
        session_id: session_id.clone(),
        websocket_url: format!("ws://localhost:8080/ws?session={}", session_id),
    })
}
