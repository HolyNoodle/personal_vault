pub mod user_aggregate;
pub mod video_session;
pub mod application_session;

// Re-exports
// pub use user_aggregate::UserAggregate;

// Legacy video session (being phased out in favor of application_session)
pub use video_session::{VideoSession, VideoSessionId, VideoConfig, VideoCodec, SessionState};

// New application-centric model
pub use application_session::{
    ApplicationSession, SessionId, AppId, SandboxedExecution,
    VideoConfig as AppVideoConfig, VideoCodec as AppVideoCodec, 
    SandboxConstraints, ResourceLimits, SessionState as AppSessionState,
};
