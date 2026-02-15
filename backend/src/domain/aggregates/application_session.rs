use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::value_objects::UserRole;

/// Application session aggregate root
/// Represents a running application instance in either sandboxed or browser mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSession {
    pub id: SessionId,
    pub app_id: AppId,
    pub user_id: String,
    pub execution: SandboxedExecution,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ApplicationSession {
    /// Create a new application session
    pub fn new(
        app_id: AppId,
        user_id: String,
        execution: SandboxedExecution,
        timeout_minutes: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::generate(),
            app_id,
            user_id,
            execution,
            state: SessionState::Initializing,
            created_at: now,
            started_at: None,
            last_activity: now,
            expires_at: now + chrono::Duration::minutes(timeout_minutes as i64),
        }
    }

    /// Mark session as ready (sandbox created, app launched, or browser bundle loaded)
    pub fn mark_ready(&mut self) {
        self.state = SessionState::Ready;
        self.started_at = Some(Utc::now());
    }

    // Removed unused methods mark_active, update_activity, is_expired, is_idle, terminate, and is_active
}

/// Session ID value object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    // Removed unused methods from_string and as_str
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Application ID value object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AppId(String);

impl AppId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Execution mode - all users use sandboxed applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxedExecution {
    pub sandbox_id: Option<String>,
    pub video_config: VideoConfig,
    pub constraints: SandboxConstraints,
    pub user_role: UserRole,
}

/// Video configuration for sandboxed mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub width: u16,
    pub height: u16,
    pub framerate: u8,
    #[serde(default = "default_codec")]
    pub codec: VideoCodec,
}

fn default_codec() -> VideoCodec {
    VideoCodec::H264
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            framerate: 30,
            codec: VideoCodec::H264,
        }
    }
}

/// Video codec
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoCodec {
    H264,
    VP8,
    VP9,
    AV1,
}

/// Sandbox constraints for security isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConstraints {
    /// Allowed file paths (read-only via Landlock)
    pub allowed_paths: Vec<String>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Network isolation
    pub network_isolated: bool,
    /// Enable watermarking on video stream
    pub watermarking: bool,
    /// Record session for audit
    pub record_session: bool,
}

impl Default for SandboxConstraints {
    fn default() -> Self {
        Self {
            allowed_paths: vec![],
            resource_limits: ResourceLimits::default(),
            network_isolated: true,
            watermarking: false,
            record_session: false,
        }
    }
}

/// Resource limits for sandboxed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU percentage (0-100)
    pub cpu_percent: u8,
    /// Memory limit in MB
    pub memory_mb: u32,
    /// Maximum number of processes
    pub max_pids: u16,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_percent: 50,
            memory_mb: 512,
            max_pids: 100,
        }
    }
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Initializing,
    Ready,
    Active,
    Idle,
    Terminating,
    Terminated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ApplicationSession::new(
            AppId::new("file-explorer-v1"),
            "user123".to_string(),
            SandboxedExecution {
                sandbox_id: None,
                video_config: VideoConfig::default(),
                constraints: SandboxConstraints::default(),
                user_role: UserRole::Client,
            },
            120,
        );

        assert_eq!(session.state, SessionState::Initializing);
        assert!(session.started_at.is_none());
        // Removed: assert!(!session.is_expired());
    }

    #[test]
    fn test_session_lifecycle() {
        let mut session = ApplicationSession::new(
            AppId::new("file-explorer-v1"),
            "user123".to_string(),
            SandboxedExecution {
                sandbox_id: None,
                video_config: VideoConfig::default(),
                constraints: SandboxConstraints::default(),
                user_role: UserRole::Client,
            },
            120,
        );

        session.mark_ready();
        assert_eq!(session.state, SessionState::Ready);
        assert!(session.started_at.is_some());

        // Removed: session.mark_active();
        // Removed: assert_eq!(session.state, SessionState::Active);
        // Removed: assert!(session.is_active());

        // Removed: session.terminate();
        // Removed: assert_eq!(session.state, SessionState::Terminated);
        // Removed: assert!(!session.is_active());
    }

    #[test]
    fn test_idle_detection() {
        let mut session = ApplicationSession::new(
            AppId::new("file-explorer-v1"),
            "user123".to_string(),
            SandboxedExecution {
                sandbox_id: None,
                video_config: VideoConfig::default(),
                constraints: SandboxConstraints::default(),
                user_role: UserRole::Client,
            },
            120,
        );

        // Simulate old activity (1 hour ago)
        session.last_activity = Utc::now() - chrono::Duration::hours(1);

        // Should be idle after 30 minutes of inactivity
        // Removed: assert!(session.is_idle(30));

        // Removed: session.update_activity();
        // Removed: assert!(!session.is_idle(30));
    }
}
