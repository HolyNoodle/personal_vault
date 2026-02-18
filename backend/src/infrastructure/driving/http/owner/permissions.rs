use axum::{extract::{State, Query, Path}, http::StatusCode, response::IntoResponse, Json};
use crate::infrastructure::AppState;
use crate::infrastructure::driving::http::middleware::auth::AuthenticatedUser;
use crate::application::owner::commands::{list_permissions, revoke_permission};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ListPermissionsQuery {
    pub client_id: Option<String>,
}

pub async fn list_permissions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListPermissionsQuery>,
) -> impl IntoResponse {
    if !user.roles.contains(&crate::domain::value_objects::user_role::UserRole::Owner) {
        return (StatusCode::FORBIDDEN, "Not an owner").into_response();
    }
    let client_id = query
        .client_id
        .as_ref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(crate::domain::value_objects::UserId::from_uuid);
    match list_permissions::execute(&*state.file_permission_repo, &user.id, client_id.as_ref()).await {
        Ok(perms) => (StatusCode::OK, Json(perms)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn revoke_permission(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(permission_id): Path<Uuid>,
) -> impl IntoResponse {
    if !user.roles.contains(&crate::domain::value_objects::user_role::UserRole::Owner) {
        return (StatusCode::FORBIDDEN, "Not an owner").into_response();
    }
    match revoke_permission::execute(&*state.file_permission_repo, &permission_id).await {
        Ok(_) => (StatusCode::OK, "Permission revoked").into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
