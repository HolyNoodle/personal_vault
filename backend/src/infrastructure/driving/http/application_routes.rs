use axum::response::IntoResponse;
use axum::http::StatusCode;
// ...existing code...
// ...existing code...
// ...existing code...

/// HTTP handler state for application routes
#[derive(Clone)]
pub struct AppHandlerState {
    // Removed launcher_service field
}

/// Request to launch application
// Removed all orphaned serde derives, attributes, and related struct definitions

// Removed all code referencing deleted struct types and handler functions

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
