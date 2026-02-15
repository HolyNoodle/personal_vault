use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::domain::aggregates::{VideoSession, VideoConfig};
use crate::infrastructure::driven::{InMemoryVideoSessionRepository, XvfbManager, FfmpegManager};
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
    streaming: Arc<FfmpegManager>,
}

impl CreateSessionHandler {
    pub fn new(
        session_repo: Arc<InMemoryVideoSessionRepository>,
        sandbox: Arc<XvfbManager>,
        streaming: Arc<FfmpegManager>,
    ) -> Self {
        Self {
            session_repo,
            sandbox,
            streaming,
        }
    }

    pub async fn handle(&self, command: CreateSessionCommand, _webrtc_adapter: Arc<WebRTCAdapter>) -> Result<CreateSessionResult> {
        // Validate config
        command.config.validate()
            .map_err(|e| anyhow::anyhow!("Invalid video config: {}", e))?;

        // Create domain object
        let session = VideoSession::new(command.user_id, command.config.clone());

        // Sandbox, streaming, and repository features disabled (trait methods removed)

        // Return result
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
    streaming: Arc<FfmpegManager>,
}

impl TerminateSessionHandler {
    pub fn new(
        session_repo: Arc<InMemoryVideoSessionRepository>,
        sandbox: Arc<XvfbManager>,
        streaming: Arc<FfmpegManager>,
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
