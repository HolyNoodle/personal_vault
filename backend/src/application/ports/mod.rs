// Application ports - Driven ports (output ports implemented by infrastructure)

pub mod user_repository;
pub mod credential_repository;
pub mod challenge_repository;
pub mod video_ports;
pub mod application_ports;

pub use user_repository::UserRepository;
pub use credential_repository::CredentialRepository;
pub use challenge_repository::ChallengeRepository;

// Legacy video ports (being phased out)
pub use video_ports::{VideoSessionRepository, VideoStreamingPort, SandboxPort};

// New application-centric ports
pub use application_ports::{
    ApplicationSessionRepository,
    ApplicationLauncherPort,
    SandboxIsolationPort,
    VideoStreamingPort as AppVideoStreamingPort,
    InputForwardingPort,
    FileSystemPort,
    ApplicationConfig,
    BrowserLaunchInfo,
    SandboxHandle,
    ResourceUsage,
    StreamHandle,
    WebRTCOffer,
    WebRTCAnswer,
    InputEvent,
    MouseButton,
    FileEntry,
    FileMetadata,
    QuotaInfo,
};
