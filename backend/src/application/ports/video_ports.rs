use anyhow::Result;
use crate::domain::aggregates::{VideoSession, VideoSessionId};
use tokio::process::ChildStdout;

/// Port for video session repository
pub trait VideoSessionRepository: Send + Sync {
    async fn save(&self, session: &VideoSession) -> Result<()>;
    async fn find_by_id(&self, id: &VideoSessionId) -> Result<Option<VideoSession>>;
    async fn find_by_user_id(&self, user_id: &str) -> Result<Vec<VideoSession>>;
    async fn delete(&self, id: &VideoSessionId) -> Result<()>;
}

/// Port for video streaming service (implemented by infrastructure)
pub trait VideoStreamingPort: Send + Sync {
    async fn start_session(&self, session_id: &VideoSessionId, display: &str) -> Result<ChildStdout>;
    async fn stop_session(&self, session_id: &VideoSessionId) -> Result<()>;
    async fn is_running(&self, session_id: &VideoSessionId) -> Result<bool>;
}

/// Port for sandbox isolation service
pub trait SandboxPort: Send + Sync {
    async fn create_display(&self, session_id: &VideoSessionId, width: u16, height: u16) -> Result<String>;
    async fn launch_application(&self, session_id: &VideoSessionId, display: &str, app: &str) -> Result<()>;
    async fn cleanup(&self, session_id: &VideoSessionId) -> Result<()>;
}
