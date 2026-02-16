use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::application::client::commands::{CreateSessionCommand, CreateSessionHandler, TerminateSessionCommand, TerminateSessionHandler};
use crate::infrastructure::driving::WebRTCAdapter;

pub struct ApiState {
    pub create_session_handler: Arc<CreateSessionHandler>,
    pub terminate_session_handler: Arc<TerminateSessionHandler>,
    pub webrtc_adapter: Arc<WebRTCAdapter>,
    pub gstreamer: Arc<crate::infrastructure::driven::sandbox::GStreamerManager>,
    pub wasm_manager: Arc<crate::infrastructure::driven::sandbox::WasmAppManager>,
}

pub fn create_video_routes(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/sessions", post(create_session))
        .route("/sessions/:id", axum::routing::delete(terminate_session))
        .with_state(state)
}

async fn create_session(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_id = payload["user_id"]
        .as_str()
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "user_id required" }))
        ))?
        .to_string();

    let command = CreateSessionCommand {
        user_id,
        config: serde_json::from_value(payload["config"].clone()).unwrap_or_default(),
        application: payload["application"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| "file-explorer".to_string()),
    };

    tracing::info!("[API] Calling CreateSessionHandler for user_id={}", command.user_id);
    match state.create_session_handler.handle(command, Arc::clone(&state.webrtc_adapter)).await {
        Ok(result) => Ok(Json(json!(result))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

async fn terminate_session(
    State(state): State<Arc<ApiState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let command = TerminateSessionCommand {
        session_id: id,
    };

    match state.terminate_session_handler.handle(command).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

async fn health_check() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub fn create_video_api_router(api_state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .nest("/api", create_video_routes(api_state))
}
