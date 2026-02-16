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
            app_id: "file-explorer".to_string(),
            name: "File Explorer".to_string(),
            description: "Browse and manage files in your sandboxed environment.".to_string(),
        },
        ApplicationMetadata {
            app_id: "xterm".to_string(),
            name: "XTerm".to_string(),
            description: "Terminal emulator for command-line access.".to_string(),
        },
        ApplicationMetadata {
            app_id: "thunar".to_string(),
            name: "Thunar".to_string(),
            description: "Lightweight file manager.".to_string(),
        },
    ];
    Json(apps)
}
use axum::extract::State;
use crate::infrastructure::driving::http::video_api::ApiState;
use axum::routing::post;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LaunchApplicationRequest {
    pub app_id: String,
    pub user_id: String,
    pub user_role: String,
    pub allowed_paths: Vec<String>,
    pub video_width: u32,
    pub video_height: u32,
    pub video_framerate: u32,
    pub enable_watermarking: bool,
    pub timeout_minutes: u32,
}

#[derive(Serialize)]
pub struct LaunchApplicationResponse {
    pub session_id: String,
}

#[debug_handler]
pub async fn launch_application(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<LaunchApplicationRequest>
) -> Json<LaunchApplicationResponse> {
    tracing::info!("[API] launch_application called: app_id={}, user_id={}, user_role={}, allowed_paths={:?}, video_width={}, video_height={}, video_framerate={}, enable_watermarking={}, timeout_minutes={}",
        payload.app_id, payload.user_id, payload.user_role, payload.allowed_paths, payload.video_width, payload.video_height, payload.video_framerate, payload.enable_watermarking, payload.timeout_minutes);

    // Generate session_id
    let session_id = uuid::Uuid::new_v4().to_string();
    let width = payload.video_width as u16;
    let height = payload.video_height as u16;
    let framerate = payload.video_framerate as u8;
    let app_name = match payload.app_id.as_str() {
        "file-explorer" => "file-explorer",
        "xterm" => "xterm",
        "thunar" => "thunar",
        _ => "file-explorer",
    };

    // Get XvfbManager and GStreamerManager from global state
    // NOTE: This assumes you have static/global managers or can access them from AppHandlerState
    // If not, you must wire them into AppHandlerState
    // Use shared Arc<XvfbManager> from application state
    let sandbox = state.xvfb_manager.clone();
    let streaming = crate::infrastructure::driven::GStreamerManager::new().expect("Failed to initialize GStreamerManager");

    // Start Xvfb
    let (display_number, display_str, dbus_address) = match sandbox.start_xvfb(&session_id, width, height).await {
        Ok(tuple) => {
            tracing::info!("[session {}] Xvfb started: display={}, dbus={}", session_id, tuple.1, tuple.2);
            tuple
        },
        Err(e) => {
            tracing::error!("[session {}] Failed to start Xvfb: {}", session_id, e);
            return Json(LaunchApplicationResponse { session_id });
        }
    };

    // Launch app
    if let Err(e) = sandbox.launch_app(&session_id, &display_str, app_name, width, height).await {
        tracing::error!("[session {}] Failed to launch app '{}': {}", session_id, app_name, e);
        return Json(LaunchApplicationResponse { session_id });
    }
    tracing::info!("[session {}] App launched successfully.", session_id);

    // Start GStreamer pipeline
    if let Err(e) = streaming.start_pipeline(&session_id, &display_str, width, height, framerate) {
        tracing::error!("[session {}] Failed to start GStreamer pipeline: {}", session_id, e);
        return Json(LaunchApplicationResponse { session_id });
    }
    tracing::info!("[session {}] GStreamer pipeline started successfully.", session_id);

    Json(LaunchApplicationResponse { session_id })
}
use axum::debug_handler;
use axum::{Json, routing::get, Router};
use std::sync::Arc;
// All legacy AppHandlerState, list_applications, and related code removed. Only Arc<ApiState> is used for application routes.
