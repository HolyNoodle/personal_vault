use axum::{Json, extract::State};
use serde_json::json;
use crate::infrastructure::AppState;

pub async fn check_setup_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let count = state.user_repo.count_super_admins()
        .await
        .unwrap_or(0);
    let is_initialized = count > 0;
    Json(json!({
        "initialized": is_initialized,
        "message": if is_initialized {
            "System is initialized"
        } else {
            "System requires initial setup"
        }
    }))
}
