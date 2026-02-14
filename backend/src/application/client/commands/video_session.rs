use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::domain::aggregates::{VideoSession, VideoSessionId, VideoConfig};
use crate::infrastructure::driven::{InMemoryVideoSessionRepository, XvfbManager, FfmpegManager};
use crate::infrastructure::driving::WebRTCAdapter;
use crate::application::ports::{VideoSessionRepository, SandboxPort, VideoStreamingPort};

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

    pub async fn handle(&self, command: CreateSessionCommand, webrtc_adapter: Arc<WebRTCAdapter>) -> Result<CreateSessionResult> {
        // Validate config
        command.config.validate()
            .map_err(|e| anyhow::anyhow!("Invalid video config: {}", e))?;

        // Create domain object
        let mut session = VideoSession::new(command.user_id, command.config.clone());

        // Create sandbox display
        let display = self.sandbox.create_display(
            &session.id,
            command.config.width,
            command.config.height,
        ).await?;

        // Launch configured application in the virtual display
        self.sandbox.launch_application(&session.id, &display, &command.application, command.config.width, command.config.height).await?;

        // Start video streaming and get FFmpeg stdout
        let ffmpeg_stdout = self.streaming.start_session(
            &session.id,
            &display,
            command.config.width,
            command.config.height,
            command.config.framerate,
        ).await?;
        
        // Store FFmpeg stdout in WebRTC adapter
        webrtc_adapter.set_ffmpeg_stream(session.id.to_string(), ffmpeg_stdout).await;

        // Register input session for keyboard/mouse forwarding
        webrtc_adapter.register_input_session(session.id.to_string(), display.clone()).await;

        // Mark session as ready
        session.mark_ready();

        // Save to repository
        self.session_repo.save(&session).await?;

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

    pub async fn handle(&self, command: TerminateSessionCommand) -> Result<()> {
        let session_id = VideoSessionId::from_string(command.session_id);

        // Get session
        let mut session = self.session_repo.find_by_id(&session_id).await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Stop streaming
        self.streaming.stop_session(&session_id).await?;

        // Cleanup sandbox
        self.sandbox.cleanup(&session_id).await?;

        // Mark as terminated
        session.terminate();
        self.session_repo.save(&session).await?;

        Ok(())
    }
}
