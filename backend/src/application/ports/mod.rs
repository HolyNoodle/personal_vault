// Application ports - Driven ports (output ports implemented by infrastructure)

pub mod user_repository;
pub mod credential_repository;
pub mod challenge_repository;
pub mod application_ports;
pub mod invitation_repository;
pub mod file_permission_repository;
pub mod session_repository;

// Removed pub use for UserRepository
pub use credential_repository::CredentialRepository;
pub use challenge_repository::ChallengeRepository;
pub use invitation_repository::InvitationRepository;
pub use file_permission_repository::FilePermissionRepository;
pub use session_repository::SessionRepository;
