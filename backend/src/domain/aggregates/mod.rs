pub mod user_aggregate;
pub mod video_session;
pub mod application_session;

// Re-exports
// pub use user_aggregate::UserAggregate;

// Legacy video session (being phased out in favor of application_session)
pub use video_session::{VideoSession, VideoSessionId, VideoConfig};

// New application-centric model
pub use application_session::{
    ApplicationSession, AppId,
    SandboxConstraints, ResourceLimits,
};
