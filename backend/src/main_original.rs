use axum::{
    routing::get,
    Router,
    response::Json,
    extract::State,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{CorsLayer, Any};
use webauthn_rs::prelude::*;

mod domain;
mod application;
mod infrastructure;

use infrastructure::AppState;
use infrastructure::driven::{PostgresUserRepository, PostgresCredentialRepository, RedisChallengeRepository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("Secure Sandbox Server starting...");
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://sandbox_user:sandbox_password@postgres:5432/sandbox_dev".to_string());
    
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    println!("Connected to database");
    
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    
    let redis = redis::Client::open(redis_url)?;
    println!("Connected to Redis");
    
    let rp_origin = Url::parse("http://localhost:5173")?;
    let rp_id = "localhost";
    
    let webauthn = Arc::new(
        WebauthnBuilder::new(rp_id, &rp_origin)?
            .rp_name("Secure Sandbox")
            .build()?
    );
    
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string());
    
    // Initialize repositories (driven adapters)
    let user_repo = Arc::new(PostgresUserRepository::new(db.clone()));
    let credential_repo = Arc::new(PostgresCredentialRepository::new(db.clone()));
    let challenge_repo = Arc::new(RedisChallengeRepository::new(redis));
    
    let state = AppState {
        webauthn,
        jwt_secret,
        user_repo,
        credential_repo,
        challenge_repo,
    };
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/setup/status", get(check_setup_status))
        .merge(infrastructure::driving::auth_routes())
        .merge(infrastructure::driving::files_routes())
        .layer(cors)
        .with_state(state);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "name": "Secure Sandbox Server",
        "version": "0.1.0",
        "status": "running"
    }))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy"
    }))
}

async fn check_setup_status(State(state): State<AppState>) -> Json<serde_json::Value> {
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
