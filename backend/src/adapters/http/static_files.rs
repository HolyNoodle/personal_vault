use axum::{
    Router,
    routing::{get, post},
    http::StatusCode,
};
use tower_http::services::ServeDir;

pub fn create_router() -> Router {
    Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .nest("/api/auth", auth_routes())
        .nest("/api/owner", owner_routes())
        .nest("/api/client", client_routes())
        .nest("/api/admin", admin_routes())
        
        // Serve static files (frontend)
        // This handles all non-API routes and serves index.html for SPA routing
        .fallback_service(
            ServeDir::new("/app/static")
                .not_found_service(ServeFile::new("/app/static/index.html"))
        )
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

// Example of how to configure in main.rs:
// 
// use tower_http::services::{ServeDir, ServeFile};
// 
// let app = Router::new()
//     .route("/api/*", /* API handlers */)
//     .fallback_service(
//         ServeDir::new("static")
//             .not_found_service(ServeFile::new("static/index.html"))
//     );
