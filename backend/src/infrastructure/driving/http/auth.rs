use axum::{
    routing::{post, get},
    Router,
    response::Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use crate::application::super_admin::commands as super_admin_commands;
use crate::infrastructure::AppState;

#[derive(Deserialize)]
pub struct InitiateRegistrationRequest {
    pub email: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct InitiateRegistrationResponse {
    pub options: webauthn_rs::prelude::CreationChallengeResponse,
    pub challenge_id: String,
}

#[derive(Deserialize)]
pub struct CompleteRegistrationRequest {
    pub challenge_id: String,
    pub credential: webauthn_rs::prelude::RegisterPublicKeyCredential,
    pub email: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct InitiateLoginRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct InitiateLoginResponse {
    pub options: webauthn_rs::prelude::RequestChallengeResponse,
    pub challenge_id: String,
}

#[derive(Deserialize)]
pub struct CompleteLoginRequest {
    pub challenge_id: String,
    pub credential: webauthn_rs::prelude::PublicKeyCredential,
    pub email: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

#[derive(Serialize)]
pub struct SetupStatusResponse {
    pub initialized: bool,
}

pub fn setup_routes() -> Router<AppState> {
    Router::new()
        .route("/api/setup/status", get(check_setup_status))
        .route("/api/setup/initiate-registration", post(initiate_registration))
        .route("/api/setup/complete-registration", post(complete_registration))
        .route("/api/auth/initiate-login", post(initiate_login))
        .route("/api/auth/complete-login", post(complete_login))
}

async fn initiate_registration(
    State(state): State<AppState>,
    Json(payload): Json<InitiateRegistrationRequest>,
) -> Result<Json<InitiateRegistrationResponse>, (StatusCode, String)> {
    // Lock setup endpoint if already initialized
    let count = state.user_repo.count_super_admins().await.unwrap_or(0);
    if count > 0 {
        return Err((StatusCode::FORBIDDEN, "Setup is locked: SuperAdmin already exists".to_string()));
    }
    let result = super_admin_commands::initiate_webauthn_registration::execute(
        &state,
        &payload.email,
        &payload.display_name,
    ).await?;
    Ok(Json(InitiateRegistrationResponse {
        options: result.options,
        challenge_id: result.challenge_id,
    }))
}

async fn complete_registration(
    State(state): State<AppState>,
    Json(payload): Json<CompleteRegistrationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Lock setup endpoint if already initialized
    let count = state.user_repo.count_super_admins().await.unwrap_or(0);
    if count > 0 {
        return Err((StatusCode::FORBIDDEN, "Setup is locked: SuperAdmin already exists".to_string()));
    }
    super_admin_commands::complete_webauthn_registration::execute(
        &state,
        &payload.challenge_id,
        payload.credential,
        &payload.email,
        &payload.display_name,
    ).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn initiate_login(
    State(state): State<AppState>,
    Json(payload): Json<InitiateLoginRequest>,
) -> Result<Json<InitiateLoginResponse>, (StatusCode, String)> {
    let result = super_admin_commands::initiate_webauthn_login::execute(&state, &payload.email).await?;
    
    Ok(Json(InitiateLoginResponse {
        options: result.options,
        challenge_id: result.challenge_id,
    }))
}

async fn complete_login(
    State(state): State<AppState>,
    Json(payload): Json<CompleteLoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let result = super_admin_commands::complete_webauthn_login::execute(
        &state,
        &payload.challenge_id,
        payload.credential,
        &payload.email,
    ).await?;
    
    Ok(Json(LoginResponse {
        token: result.token,
        user: UserInfo {
            id: result.user_id,
            email: result.email,
            display_name: result.display_name,
            roles: result.roles,
        },
    }))
}
async fn check_setup_status(
    State(state): State<AppState>,
) -> Result<Json<SetupStatusResponse>, (StatusCode, String)> {
    // Check if any super admins exist in the database
    let count = state.user_repo.count_super_admins()
        .await
        .unwrap_or(0);
    let is_initialized = count > 0;
    Ok(Json(SetupStatusResponse {
        initialized: is_initialized,
    }))
}