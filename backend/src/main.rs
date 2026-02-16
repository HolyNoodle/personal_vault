use infrastructure::driven::user_repository::PostgresUserRepository;
use infrastructure::AppState;
use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

mod domain;
mod application;
mod infrastructure;

use infrastructure::driving::{WebRTCAdapter};
use infrastructure::driven::{WasmAppManager, GStreamerManager, InMemoryVideoSessionRepository, IpcSocketServer};
use infrastructure::driven::persistence::{PostgresCredentialRepository, RedisChallengeRepository};
use infrastructure::driving::http::video_api::{ApiState, create_video_api_router};
use axum::routing::post;
use infrastructure::driving::http::auth;
use application::client::commands::{CreateSessionHandler, TerminateSessionHandler};
use application::ports::{CredentialRepository, ChallengeRepository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("Sandbox Server starting...");

    // Check prerequisites
    println!("Checking prerequisites...");
    check_prerequisites()?;

    // Initialize WebAuthn
    let rp_id = std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".to_string());
    let rp_origin = std::env::var("WEBAUTHN_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let webauthn = Arc::new(
        webauthn_rs::WebauthnBuilder::new(&rp_id, &url::Url::parse(&rp_origin).unwrap())
            .unwrap()
            .rp_name("Secure Sandbox")
            .build()
            .unwrap()
    );

    // Initialize database connection pool
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://sandbox_user:sandbox_dev_password@postgres:5432/sandbox_dev".to_string());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    // Run migrations
    println!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    println!("Database migrations completed");

    // Initialize auth repositories
    let user_repo = Arc::new(
        PostgresUserRepository::new(pool.clone())
    ) as Arc<dyn crate::application::ports::user_repository::UserRepository>;
    let credential_repo = Arc::new(PostgresCredentialRepository::new(pool)) as Arc<dyn CredentialRepository>;

    // Initialize Redis challenge repository
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url)
        .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;
    let challenge_repo = Arc::new(RedisChallengeRepository::new(redis_client)) as Arc<dyn ChallengeRepository>;

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev_secret_key_change_in_production".to_string());

    // Create auth app state
    let app_state = AppState {
        webauthn,
        jwt_secret,
        user_repo,
        credential_repo,
        challenge_repo,
    };

    // Initialize infrastructure adapters
    let session_repo = Arc::new(InMemoryVideoSessionRepository::new());
    let wasm_dir = std::env::var("WASM_DIR").unwrap_or_else(|_| "/app/wasm".to_string());
    let wasm_manager = Arc::new(WasmAppManager::new(wasm_dir));
    let streaming = Arc::new(GStreamerManager::new().expect("Failed to initialize GStreamerManager"));

    // Initialize command handlers
    let create_session_handler = Arc::new(CreateSessionHandler::new(
        session_repo.clone(),
        wasm_manager.clone(),
        streaming.clone(),
    ));

    let terminate_session_handler = Arc::new(TerminateSessionHandler::new(
        session_repo.clone(),
        wasm_manager.clone(),
        streaming.clone(),
    ));

    // Initialize WebRTC adapter with WasmAppManager
    let webrtc_adapter = Arc::new(WebRTCAdapter::new(wasm_manager.clone()));

    // Create API state
    let api_state = Arc::new(ApiState {
        create_session_handler,
        terminate_session_handler,
        webrtc_adapter: Arc::clone(&webrtc_adapter),
        gstreamer: streaming.clone(),
        wasm_manager: wasm_manager.clone(),
    });

    // Build router
    let webrtc_clone = Arc::clone(&webrtc_adapter);

    // Start IPC socket server for app communication
    let ipc_socket_path = std::env::var("IPC_SOCKET_PATH")
        .unwrap_or_else(|_| "/tmp/sandbox-ipc.sock".to_string());
    let ipc_server = Arc::new(IpcSocketServer::new(ipc_socket_path.clone().into()));
    let ipc_server_clone = ipc_server.clone();

    tokio::spawn(async move {
        if let Err(e) = ipc_server_clone.start().await {
            tracing::error!("IPC server error: {}", e);
        }
    });
    println!("IPC socket server started at {}", ipc_socket_path);

    // Auth routes with AppState
    let auth_routes = auth::setup_routes()
        .with_state(app_state.clone());

    // WebSocket route with WebRTCAdapter state
    let ws_routes = Router::new()
        .route("/ws", get(infrastructure::driving::webrtc::ws_handler))
        .with_state(webrtc_clone);

    // Application platform routes
    let app_routes = Router::new()
        .route("/api/applications", get(infrastructure::driving::http::application_routes::list_applications))
        .route("/api/applications/launch", post(infrastructure::driving::http::application_routes::launch_application))
        .with_state(api_state.clone());

    // Merge all routes
    let app = Router::new()
        .merge(auth_routes)
        .merge(ws_routes)
        .merge(create_video_api_router(api_state))
        .merge(app_routes)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Server listening on http://{}", addr);
    println!("\nAvailable Endpoints:");
    println!("   AUTH:");
    println!("   - POST http://localhost:8080/api/auth/register");
    println!("   - POST http://localhost:8080/api/auth/login");
    println!("   APPLICATION PLATFORM:");
    println!("   - GET  http://localhost:8080/api/applications");
    println!("   - POST http://localhost:8080/api/applications/launch");
    println!("   VIDEO:");
    println!("   - POST http://localhost:8080/api/sessions");
    println!("   - WS   ws://localhost:8080/ws");
    println!("   SYSTEM:");
    println!("   - GET  http://localhost:8080/health");
    println!();


    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn check_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    // Check GStreamer
    if gstreamer::init().is_err() {
        eprintln!("GStreamer not available. Install GStreamer runtime and plugins.");
        return Err("GStreamer not available".into());
    }
    println!("GStreamer found");

    Ok(())
}
