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
use infrastructure::driven::persistence::{SqliteCredentialRepository, RedisChallengeRepository, SqliteUserRepository, SqliteInvitationRepository, SqliteFilePermissionRepository, SqliteSessionRepository};
use axum::routing::post;
use infrastructure::driving::http::auth;
use application::ports::{CredentialRepository, ChallengeRepository, InvitationRepository, FilePermissionRepository, SessionRepository};

use diesel::r2d2::{self, ConnectionManager};
use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

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

    // Initialize SQLite database connection pool
    let storage_path = std::env::var("STORAGE_PATH").unwrap();
    let db_path = format!("{}/internal/db/sandbox.db", storage_path);

    // Create folder if not exist
    std::fs::create_dir_all(std::path::Path::new(&db_path).parent().unwrap_or(std::path::Path::new("/data/db")))
          .map_err(|e| anyhow::anyhow!("Failed to create database directory: {}", e))?;

    // Ensure parent directory exists
    std::fs::create_dir_all(
        std::path::Path::new(&db_path).parent().unwrap_or(std::path::Path::new("/data/db"))
    ).map_err(|e| anyhow::anyhow!("Failed to create database directory: {}", e))?;

    let manager = ConnectionManager::<SqliteConnection>::new(&db_path);
    let pool = r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| anyhow::anyhow!("Failed to create database pool: {}", e))?;

    // Run embedded migrations
    println!("Running database migrations...");
    let mut conn = pool.get()
        .map_err(|e| anyhow::anyhow!("Failed to get DB connection: {}", e))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;
    drop(conn);
    println!("Database migrations completed");

    let pool = Arc::new(pool);

    // Initialize auth repositories
    let user_repo = Arc::new(SqliteUserRepository::new(pool.clone()))
        as Arc<dyn crate::application::ports::user_repository::UserRepository>;
    let credential_repo = Arc::new(SqliteCredentialRepository::new(pool.clone()))
        as Arc<dyn CredentialRepository>;
    let invitation_repo = Arc::new(SqliteInvitationRepository::new(pool.clone()))
        as Arc<dyn InvitationRepository>;
    let file_permission_repo = Arc::new(SqliteFilePermissionRepository::new(pool.clone()))
        as Arc<dyn FilePermissionRepository>;
    let session_repo = Arc::new(SqliteSessionRepository::new(pool))
        as Arc<dyn SessionRepository>;

    // Initialize Redis challenge repository
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url)
        .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;
    let challenge_repo = Arc::new(RedisChallengeRepository::new(redis_client)) as Arc<dyn ChallengeRepository>;

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev_secret_key_change_in_production".to_string());

    // Initialize Xvfb manager
    let apps_root = std::env::var("APPS_ROOT").unwrap_or_else(|_| "/app/.app".to_string());
    let xvfb_manager = Arc::new(XvfbManager::new(apps_root));

    // Initialize WebRTC adapter with XvfbManager
    let webrtc_adapter = Arc::new(WebRTCAdapter::new(xvfb_manager.clone()));

    // Create auth app state
    let app_state = AppState {
        webauthn,
        jwt_secret,
        user_repo,
        credential_repo,
        challenge_repo,
        invitation_repo,
        file_permission_repo,
        session_repo,
        xvfb_manager: xvfb_manager.clone(),
        storage_path: storage_path.clone(),
    };

    // Create API state
    // ApiState and video session handlers removed

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

    // WebSocket route with WebRTCAdapter state + AppState extension for session tracking
    let ws_routes = Router::new()
        .route("/ws", get(infrastructure::driving::webrtc::ws_handler))
        .layer(axum::Extension(app_state.clone()))
        .with_state(webrtc_adapter.clone());

    // Application platform routes (require auth — enforced in launch_application handler)
    let app_routes = Router::new()
        .route("/api/applications", get(infrastructure::driving::http::application_routes::list_applications))
        .route("/api/applications/launch", post(infrastructure::driving::http::application_routes::launch_application))
        .with_state(app_state.clone());

    use infrastructure::driving::http::{owner, client, invite};
    // Owner routes (require Owner role — enforced in handlers)
    let owner_routes = Router::new()
        .route("/api/invitations", post(owner::invitations::create_invitation))
        .route("/api/permissions", get(owner::permissions::list_permissions))
        .route("/api/permissions/{id}", axum::routing::delete(owner::permissions::revoke_permission))
        .with_state(app_state.clone());

    // Client routes (require Client role — enforced in handlers)
    let client_routes = Router::new()
        .route("/api/my-permissions", get(client::my_permissions::list_my_permissions))
        .with_state(app_state.clone());

    // Invite routes (public)
    let invite_routes = Router::new()
        .route("/api/invitations/{token}", get(invite::view::view_invitation))
        .route("/api/invitations/{token}/accept/initiate", post(invite::initiate::initiate_webauthn_registration))
        .route("/api/invitations/{token}/accept/complete", post(invite::complete::complete_webauthn_registration))
        .with_state(app_state.clone());

    // Custom middleware: 503 if not initialized and not /api/setup/* or /health
    use axum::{middleware::Next, http::{Request, StatusCode}, response::Response, body::Body};

    async fn require_initialized(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
        let path = req.uri().path();
        // Allow setup and health endpoints always
        if path.starts_with("/api/setup/") || path == "/api/setup/status" || path == "/health" {
            return Ok(next.run(req).await);
        }
        // Check initialized state
        let state = req.extensions().get::<AppState>().cloned();
        if let Some(state) = state {
            let count = state.user_repo.count_super_admins().await.unwrap_or(0);
            if count == 0 {
                return Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(axum::body::Body::from("Service unavailable: system not initialized"))
                    .unwrap());
            }
        }
        Ok(next.run(req).await)
    }

    let app = Router::new()
        .merge(auth_routes)
        .merge(ws_routes)
        .merge(app_routes)
        .merge(owner_routes)
        .merge(client_routes)
        .merge(invite_routes)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .layer(axum::middleware::from_fn(require_initialized));

    // Background task: clean up expired sessions every 60 seconds
    {
        let state_for_expiry = app_state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                match state_for_expiry.session_repo.find_expired().await {
                    Ok(expired) => {
                        for session in expired {
                            let sid = session.id.to_string();
                            let _ = state_for_expiry.xvfb_manager.cleanup_session(&sid).await;
                            let _ = state_for_expiry.session_repo.terminate(&session.id).await;
                            tracing::info!("Expired session cleaned up: {}", sid);
                        }
                    }
                    Err(e) => tracing::warn!("Failed to query expired sessions: {}", e),
                }
            }
        });
    }

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
