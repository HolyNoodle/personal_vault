use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crate::infrastructure::AppState;
use crate::infrastructure::driving::http::middleware::auth::AuthenticatedUser;
use crate::application::client::commands::list_my_permissions;

pub async fn list_my_permissions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    if !user.roles.contains(&crate::domain::value_objects::user_role::UserRole::Client) {
        return (StatusCode::FORBIDDEN, "Not a client").into_response();
    }
    match list_my_permissions::execute(&*state.file_permission_repo, &user.id).await {
        Ok(perms) => (StatusCode::OK, Json(perms)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
