use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::client::commands::{
    LaunchApplicationCommand, ApplicationLauncherService,
};

/// HTTP handler state for application routes
#[derive(Clone)]
pub struct AppHandlerState {
    pub launcher_service: Arc<ApplicationLauncherService>,
}

/// Request to launch application
#[derive(Debug, Deserialize)]
pub struct LaunchAppRequest {
    pub app_id: String,
    pub user_id: String, // In production, this would come from JWT
    pub user_role: String, // "owner" or "client"
    pub allowed_paths: Vec<String>,
    #[serde(default = "default_width")]
    pub video_width: u16,
    #[serde(default = "default_height")]
    pub video_height: u16,
    #[serde(default = "default_framerate")]
    pub video_framerate: u8,
    #[serde(default)]
    pub enable_watermarking: bool,
    #[serde(default = "default_timeout")]
    pub timeout_minutes: u32,
}

fn default_timeout() -> u32 {
    120
}

fn default_width() -> u16 {
    1920
}
fn default_height() -> u16 {
    1080
}
fn default_framerate() -> u8 {
    30
}

/// Response from launching application
#[derive(Debug, Serialize)]
pub struct LaunchAppResponse {
    pub session_id: String,
    pub webrtc_offer: String,
}

/// Application metadata for listing
#[derive(Debug, Serialize)]
pub struct ApplicationInfo {
    pub app_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Response from listing applications
#[derive(Debug, Serialize)]
pub struct ListApplicationsResponse {
    pub applications: Vec<ApplicationInfo>,
}

/// GET /api/applications
pub async fn list_applications() -> Result<impl IntoResponse, AppError> {
    // For now, return hardcoded list. In future, this would come from a registry
    let applications = vec![
        ApplicationInfo {
            app_id: "file-explorer-v1".to_string(),
            name: "File Explorer".to_string(),
            description: "Browse and preview files (PDF, images, videos) in a secure sandboxed environment".to_string(),
            version: "1.0.0".to_string(),
        },
    ];

    Ok(Json(ListApplicationsResponse { applications }))
}

/// POST /api/applications/launch
pub async fn launch_application(
    State(state): State<AppHandlerState>,
    Json(request): Json<LaunchAppRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!("📨 Received launch application request:");
    println!("  └─ App: {}", request.app_id);
    println!("  └─ User: {} ({})", request.user_id, request.user_role);
    println!("  └─ Mode: Sandboxed {}x{}@{}fps", request.video_width, request.video_height, request.video_framerate);

    // Parse user role
    let user_role = match request.user_role.to_lowercase().as_str() {
        "owner" => crate::domain::value_objects::UserRole::Owner,
        "client" => crate::domain::value_objects::UserRole::Client,
        _ => return Err(AppError(anyhow::anyhow!("Invalid user_role. Must be 'owner' or 'client'"))),
    };

    let command = LaunchApplicationCommand {
        app_id: request.app_id,
        user_id: request.user_id,
        user_role,
        allowed_paths: request.allowed_paths,
        video_width: request.video_width,
        video_height: request.video_height,
        video_framerate: request.video_framerate,
        enable_watermarking: request.enable_watermarking,
        timeout_minutes: request.timeout_minutes,
    };

    let response = state.launcher_service.execute(command).await?;

    Ok(Json(LaunchAppResponse {
        session_id: response.session_id,
        webrtc_offer: response.webrtc_offer,
    }))
}

/// Error handler
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = format!("Error: {}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
