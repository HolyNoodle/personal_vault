use anyhow::Result;
use crate::domain::aggregates::VideoSessionId;
// ...existing code...

/// Port for video session repository
pub trait VideoSessionRepository: Send + Sync {
    // Removed all unused methods from VideoSessionRepository trait
}

/// Port for video streaming service (implemented by infrastructure)
pub trait VideoStreamingPort: Send + Sync {
    // Removed all unused methods from VideoStreamingPort trait
}

/// Port for sandbox isolation service
pub trait SandboxPort: Send + Sync {
    async fn create_display(&self, session_id: &VideoSessionId, width: u16, height: u16) -> Result<String>;
    async fn launch_application(&self, session_id: &VideoSessionId, display: &str, app: &str, width: u16, height: u16) -> Result<()>;
    async fn cleanup(&self, session_id: &VideoSessionId) -> Result<()>;
}
