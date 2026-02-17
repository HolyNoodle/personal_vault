use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::domain::aggregates::{VideoSession, VideoConfig};
use crate::infrastructure::driven::NativeAppManager;
use crate::infrastructure::driving::WebRTCAdapter;

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
    "file-explorer".to_string()
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResult {
    pub session_id: String,
    pub websocket_url: String,
}

/// Command handler for creating video sessions
pub struct CreateSessionHandler;

impl CreateSessionHandler {
    pub fn new(_: Arc<NativeAppManager>) -> Self {
        Self
    }

    pub async fn handle(&self, command: CreateSessionCommand, _webrtc_adapter: Arc<WebRTCAdapter>) -> Result<CreateSessionResult> {
        tracing::info!("[session {}] Creating video session for app '{}'", command.user_id, command.application);

        // Validate config
        if let Err(e) = command.config.validate() {
            return Err(anyhow::anyhow!("Invalid video config: {}", e));
        }

        // Create domain object
        let session = VideoSession::new(command.user_id.clone(), command.config.clone());
        let session_id = session.id.to_string();

        // The WASM app and GStreamer pipeline are launched when the WebRTC
        // connection is established (on request-offer), not here.
        // This handler just creates the session record.

        tracing::info!("[session {}] Session created, ready for WebRTC connection", session_id);

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
    native_manager: Arc<NativeAppManager>,
}

impl TerminateSessionHandler {
    pub fn new(
        native_manager: Arc<NativeAppManager>,
    ) -> Self {
        Self {
            native_manager,
        }
    }

    pub async fn handle(&self, command: TerminateSessionCommand) -> Result<()> {
        tracing::info!("Terminating session: {}", command.session_id);
        self.native_manager.cleanup_session(&command.session_id).await?;
        Ok(())
    }
}
