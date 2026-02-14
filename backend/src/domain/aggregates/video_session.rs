use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Video session aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSession {
    pub id: VideoSessionId,
    pub user_id: String, // Reference to user
    pub config: VideoConfig,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl VideoSession {
    /// Create a new video session
    pub fn new(user_id: String, config: VideoConfig) -> Self {
        Self {
            id: VideoSessionId::generate(),
            user_id,
            config,
            state: SessionState::Initializing,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
        }
    }

    /// Mark session as ready
    pub fn mark_ready(&mut self) {
        self.state = SessionState::Ready;
        self.started_at = Some(Utc::now());
    }

    /// Mark session as active (WebRTC connected)
    pub fn mark_active(&mut self) {
        self.state = SessionState::Active;
    }

    /// Terminate the session
    pub fn terminate(&mut self) {
        self.state = SessionState::Terminated;
        self.ended_at = Some(Utc::now());
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Initializing,
    Ready,
    Active,
    Terminated,
}

/// Video session ID value object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VideoSessionId(String);

impl VideoSessionId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VideoSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Video configuration value object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub width: u16,
    pub height: u16,
    pub framerate: u8,
    pub codec: VideoCodec,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    VP8,
    VP9,
}

impl VideoConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.width < 640 || self.width > 3840 {
            return Err("Width must be between 640 and 3840");
        }
        if self.height < 480 || self.height > 2160 {
            return Err("Height must be between 480 and 2160");
        }
        if self.framerate < 15 || self.framerate > 60 {
            return Err("Framerate must be between 15 and 60");
        }
        Ok(())
    }
}
