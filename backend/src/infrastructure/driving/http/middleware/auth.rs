use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::domain::value_objects::UserId;
use crate::domain::value_objects::user_role::UserRole;
use crate::infrastructure::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: UserId,
    pub email: String,
    pub roles: Vec<UserRole>,
}

#[derive(Debug, serde::Deserialize)]
struct Claims {
    sub: String,
    email: String,
    roles: Vec<String>,
    exp: usize,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, String);

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = extract(parts, state);
        std::future::ready(result)
    }
}

fn extract(parts: &mut Parts, state: &AppState) -> Result<AuthenticatedUser, (StatusCode, String)> {
    let auth_header = parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header".to_string()))?;

    let token_data = decode::<Claims>(
        auth_header,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;

    let claims = token_data.claims;
    let id = UserId::from_uuid(
        uuid::Uuid::parse_str(&claims.sub)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid user id in token".to_string()))?,
    );
    let roles = claims
        .roles
        .iter()
        .filter_map(|r| match r.as_str() {
            "super_admin" => Some(UserRole::SuperAdmin),
            "owner" => Some(UserRole::Owner),
            "client" => Some(UserRole::Client),
            _ => None,
        })
        .collect();

    Ok(AuthenticatedUser {
        id,
        email: claims.email,
        roles,
    })
}
