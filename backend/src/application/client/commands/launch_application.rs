use axum::http::StatusCode;
use crate::infrastructure::AppState;
use crate::infrastructure::driving::http::middleware::auth::AuthenticatedUser;
use crate::domain::value_objects::user_role::UserRole;
use crate::domain::entities::session::Session;

pub struct LaunchResult {
    pub session_id: String,
    pub websocket_url: String,
}

pub async fn execute(
    state: &AppState,
    user: &AuthenticatedUser,
    app_id: &str,
    width: u16,
    height: u16,
) -> Result<LaunchResult, (StatusCode, String)> {
    let ws_base = std::env::var("WEBSOCKET_BASE_URL")
        .unwrap_or_else(|_| "ws://localhost:8080".to_string());

    let session_timeout = std::env::var("SESSION_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3600);

    // Determine root_path and role context
    let (root_path, acting_as_owner_id, active_role, allowed_paths) =
        if user.roles.contains(&UserRole::Owner) || user.roles.contains(&UserRole::SuperAdmin) {
            let path = format!("{}/{}", state.storage_path, user.id);
            (path, None, "owner".to_string(), vec![])
        } else {
            let permissions = state
                .file_permission_repo
                .find_active_for_client(&user.id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

            if permissions.is_empty() {
                return Err((StatusCode::FORBIDDEN, "No active permissions for this client".to_string()));
            }

            let owner_id = permissions[0].owner_id.clone();
            let root = format!("{}/{}", state.storage_path, owner_id);
            let allowed = permissions
                .iter()
                .map(|p| format!("{}/{}", root, p.path))
                .collect::<Vec<_>>();

            (root, Some(owner_id), "client".to_string(), allowed)
        };

    // Create session record to get the session_id
    let session = Session::new(
        user.id.clone(),
        acting_as_owner_id,
        active_role,
        app_id.to_string(),
        None, // display_number set after xvfb starts
        session_timeout,
    );
    let session_id = session.id.to_string();

    state
        .session_repo
        .save(&session)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Start Xvfb using the session_id
    let start_result = state.xvfb_manager.start_xvfb(&session_id, width, height).await;
    if let Err(e) = start_result {
        let _ = state.session_repo.terminate(&session.id).await;
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to start Xvfb: {e}")));
    }

    // Launch app
    let launch_result = state
        .xvfb_manager
        .launch_app(&session_id, app_id, width, height, &root_path, &allowed_paths)
        .await;
    if let Err(e) = launch_result {
        let _ = state.xvfb_manager.cleanup_session(&session_id).await;
        let _ = state.session_repo.terminate(&session.id).await;
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to launch app: {e}")));
    }

    // Mark session ready
    let _ = state.session_repo.update_state(&session.id, "ready").await;

    let websocket_url = format!("{}/ws?session={}", ws_base, session_id);
    Ok(LaunchResult { session_id, websocket_url })
}
