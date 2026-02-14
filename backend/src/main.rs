use axum::{
    routing::get,
    Router,
    extract::{WebSocketUpgrade, State},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

mod domain;
mod application;
mod infrastructure;

use infrastructure::driving::{WebRTCAdapter};
use infrastructure::driven::{XvfbManager, FfmpegManager, InMemoryVideoSessionRepository};
use infrastructure::driving::http::video_api::{ApiState, create_poc_router};
use application::client::commands::{CreateSessionHandler, TerminateSessionHandler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🚀 WebRTC POC Server starting...");
    
    // Check prerequisites
    println!("Checking prerequisites...");
    check_prerequisites()?;
    
    // Initialize infrastructure adapters (hexagonal architecture - driven adapters)
    let session_repo = Arc::new(InMemoryVideoSessionRepository::new());
    let sandbox = Arc::new(XvfbManager::new());
    let streaming = Arc::new(FfmpegManager::new());
    
    // Initialize application layer (command handlers)
    let create_session_handler = Arc::new(CreateSessionHandler::new(
        session_repo.clone(),
        sandbox.clone(),
        streaming.clone(),
    ));
    
    let terminate_session_handler = Arc::new(TerminateSessionHandler::new(
        session_repo.clone(),
        sandbox.clone(),
        streaming.clone(),
    ));
    
    // Initialize driving adapters
    let webrtc_adapter = Arc::new(WebRTCAdapter::new());
    
    // Create API state
    let api_state = Arc::new(ApiState {
        create_session_handler,
        terminate_session_handler,
    });
    
    // Build router (hexagonal architecture - driving adapters)
    let webrtc_clone = Arc::clone(&webrtc_adapter);
    let app = Router::new()
        .route("/ws", get(|ws: WebSocketUpgrade, State(adapter): State<Arc<WebRTCAdapter>>| async move {
            use infrastructure::driving::webrtc::handle_socket_internal;
            ws.on_upgrade(move |socket| handle_socket_internal(socket, adapter))
        }))
        .with_state(webrtc_clone)
        .merge(create_poc_router(api_state))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );
    
    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("✅ Server listening on http://{}", addr);
    println!("\n📋 POC Endpoints:");
    println!("   - POST http://localhost:8080/api/sessions - Create video session");
    println!("   - WS   ws://localhost:8080/ws - WebRTC signaling");
    println!("   - GET  http://localhost:8080/health - Health check");
    println!("\n⚠️  This is a POC build - authentication disabled for testing\n");
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn check_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    // Check Xvfb
    if !Command::new("Xvfb").arg("-help").output().is_ok() {
        eprintln!("❌ Xvfb not found. Install with: sudo apt-get install xvfb");
        return Err("Xvfb not installed".into());
    }
    println!("✅ Xvfb found");
    
    // Check FFmpeg
    if !Command::new("ffmpeg").arg("-version").output().is_ok() {
        eprintln!("❌ FFmpeg not found. Install with: sudo apt-get install ffmpeg");
        return Err("FFmpeg not installed".into());
    }
    println!("✅ FFmpeg found");
    
    // Check xterm (optional, for demo app)
    if !Command::new("xterm").arg("-version").output().is_ok() {
        println!("⚠️  xterm not found (optional). Install with: sudo apt-get install xterm");
    } else {
        println!("✅ xterm found");
    }
    
    Ok(())
}
