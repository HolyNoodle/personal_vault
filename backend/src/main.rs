use infrastructure::driven::user_repository::PostgresUserRepository;
use infrastructure::AppState;
use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

use tracing::info;

mod domain;
mod application;
mod infrastructure;

use infrastructure::driving::{WebRTCAdapter};
use infrastructure::driven::{XvfbManager, IpcSocketServer};
use infrastructure::driven::persistence::{PostgresCredentialRepository, RedisChallengeRepository};
use axum::routing::post;
use infrastructure::driving::http::auth;
use application::ports::{CredentialRepository, ChallengeRepository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("[DEBUG] Backend main() started");
    // Set panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("[PANIC] {}", panic_info);
    }));
    // Log on shutdown (SIGINT/SIGTERM)
    if let Err(e) = ctrlc::set_handler(move || {
        tracing::warn!("[SHUTDOWN] Received termination signal (SIGINT/SIGTERM) - ctrlc handler");
        // Print backtrace if possible
        let bt = std::backtrace::Backtrace::capture();
        tracing::warn!("[SHUTDOWN] Backtrace: {:?}", bt);
        std::process::exit(0);
    }) {
        tracing::error!("[SHUTDOWN] Failed to set Ctrl-C handler: {}", e);
    }
    // Install default crypto provider for jsonwebtoken crate

    // Minimal logging: info and above
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
    let apps_root = std::env::var("APPS_ROOT").unwrap_or_else(|_| "/app/.app".to_string());
    let xvfb_manager = Arc::new(XvfbManager::new(apps_root));


    // Initialize WebRTC adapter with XvfbManager
    let webrtc_adapter = Arc::new(WebRTCAdapter::new(xvfb_manager.clone()));

    // Create API state
    // ApiState and video session handlers removed

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
        .route("/api/applications/launch", post(infrastructure::driving::http::application_routes::launch_application));

    // Merge all routes
    let app = Router::new()
        .merge(auth_routes)
        .merge(ws_routes)
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
    println!("   WEBSOCKET:");
    println!("   - WS   ws://localhost:8080/ws (for application signaling, not video)");
    println!("   SYSTEM:");
    println!("   - GET  http://localhost:8080/health");
    println!();


    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("[DEBUG] TcpListener bound on {}", addr);
    info!("[DEBUG] About to call axum::serve");
    tracing::warn!("[AXUM] >>> axum::serve about to start");
    let serve_result = axum::serve(listener, app).await;
    tracing::warn!("[AXUM] <<< axum::serve returned: {:?}", serve_result);
    if let Err(ref e) = serve_result {
        tracing::error!("[SHUTDOWN] axum::serve returned error: {:?}", e);
    } else {
        tracing::warn!("[SHUTDOWN] axum::serve returned Ok, server shutting down");
    }
    tracing::warn!("[SHUTDOWN] main() is returning, backend will exit");
    serve_result?;

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
