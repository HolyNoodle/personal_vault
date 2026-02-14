use axum::{
    routing::{get, post},
    Router,
    response::Json,
    extract::State,
    http::StatusCode,
};
use crate::infrastructure::{AppState, AuthUser};

pub fn files_routes() -> Router<AppState> {
    Router::new()
        .route("/api/files", get(list_files))
        .route("/api/files/upload", post(upload_file))
}

async fn list_files(
    auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    println!("Listing files for user: {} ({})", auth.email, auth.user_id);
    
    Ok(Json(serde_json::json!({
        "files": []
    })))
}

async fn upload_file(
    auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    println!("File upload requested by user: {} ({})", auth.email, auth.user_id);
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "File storage not yet implemented"
    })))
}
