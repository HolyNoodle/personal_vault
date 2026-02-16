use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::domain::aggregates::{VideoSession, VideoConfig};
use crate::infrastructure::driven::{InMemoryVideoSessionRepository, XvfbManager, GStreamerManager};
use crate::infrastructure::driving::WebRTCAdapter;
// Removed imports for deleted traits

/// Command to create a new video session
#[derive(Debug, Deserialize)]
pub struct CreateSessionCommand {
    pub user_id: String,
    #[serde(default)]
    pub config: VideoConfig,
    #[serde(default = "default_application")]
    pub application: String,
}

fn default_application() -> String {
    "xterm".to_string()
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResult {
    pub session_id: String,
    pub websocket_url: String,
}

/// Command handler for creating video sessions
pub struct CreateSessionHandler {
    session_repo: Arc<InMemoryVideoSessionRepository>,
    sandbox: Arc<XvfbManager>,
    streaming: Arc<GStreamerManager>,
}

impl CreateSessionHandler {
    pub fn new(
        session_repo: Arc<InMemoryVideoSessionRepository>,
        sandbox: Arc<XvfbManager>,
        streaming: Arc<GStreamerManager>,
    ) -> Self {
        Self {
            session_repo,
            sandbox,
            streaming,
        }
    }

    pub async fn handle(&self, command: CreateSessionCommand, _webrtc_adapter: Arc<WebRTCAdapter>) -> Result<CreateSessionResult> {
        tracing::info!("[session {}] Entered CreateSessionHandler::handle", command.user_id);
        tracing::info!("[session {}] Input config: width={}, height={}, framerate={}, application={}", command.user_id, command.config.width, command.config.height, command.config.framerate, command.application);

        // Validate config
        if let Err(e) = command.config.validate() {
            tracing::error!("[session {}] Invalid video config: {}", command.user_id, e);
            return Err(anyhow::anyhow!("Invalid video config: {}", e));
        }

        // Create domain object
        let session = VideoSession::new(command.user_id.clone(), command.config.clone());
        let session_id = session.id.to_string();

        tracing::info!("[session {}] [PRE-STEP] About to start Xvfb (width={}, height={})", session_id, command.config.width, command.config.height);
        let (display_number, display_str, dbus_address) = match self.sandbox.start_xvfb(
            &session_id,
            command.config.width,
            command.config.height
        ).await {
            Ok(tuple) => {
                tracing::info!("[session {}] Xvfb started: display={}, dbus={}", session_id, tuple.1, tuple.2);
                tuple
            },
            Err(e) => {
                tracing::error!("[session {}] Failed to start Xvfb: {}", session_id, e);
                return Err(anyhow::anyhow!("Failed to start Xvfb: {}", e));
            }
        };

        tracing::info!("[session {}] [PRE-STEP] About to launch app '{}' on display {} (width={}, height={})", session_id, command.application, display_str, command.config.width, command.config.height);
        if let Err(e) = self.sandbox.launch_app(
            &session_id,
            &display_str,
            &command.application,
            command.config.width,
            command.config.height
        ).await {
            tracing::error!("[session {}] Failed to launch app '{}': {}", session_id, command.application, e);
            return Err(anyhow::anyhow!("Failed to launch app: {}", e));
        }
        tracing::info!("[session {}] App launched successfully.", session_id);

        tracing::info!("[session {}] [PRE-STEP] About to start GStreamer pipeline on display {} (width={}, height={}, framerate={})", session_id, display_str, command.config.width, command.config.height, command.config.framerate);
        tracing::info!("[session {}] [LOG-ENSURE] GStreamerManager will use display_str='{}'", session_id, display_str);
        if let Err(e) = self.streaming.start_pipeline(
            &session.id.to_string(),
            &display_str,
            command.config.width,
            command.config.height,
            command.config.framerate
        ) {
            tracing::error!("[session {}] Failed to start GStreamer pipeline: {}", session_id, e);
            return Err(anyhow::anyhow!("Failed to start GStreamer pipeline: {}", e));
        }
        tracing::info!("[session {}] GStreamer pipeline started successfully.", session_id);

        Ok(CreateSessionResult {
            session_id: session.id.to_string(),
            websocket_url: format!("ws://localhost:8080/ws?session={}", session.id),
        })
    }
}

/// Command to terminate a session
#[derive(Debug, Deserialize)]
pub struct TerminateSessionCommand {
    pub session_id: String,
}

pub struct TerminateSessionHandler {
    session_repo: Arc<InMemoryVideoSessionRepository>,
    sandbox: Arc<XvfbManager>,
    streaming: Arc<GStreamerManager>,
}

impl TerminateSessionHandler {
    pub fn new(
        session_repo: Arc<InMemoryVideoSessionRepository>,
        sandbox: Arc<XvfbManager>,
        streaming: Arc<GStreamerManager>,
    ) -> Self {
        Self {
            session_repo,
            sandbox,
            streaming,
        }
    }

    pub async fn handle(&self, _command: TerminateSessionCommand) -> Result<()> {
        // Terminate session features disabled (trait methods removed)
        Ok(())
    }
}
