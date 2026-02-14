// Application ports - Driven ports (output ports implemented by infrastructure)

pub mod user_repository;
pub mod credential_repository;
pub mod challenge_repository;

pub use user_repository::UserRepository;
pub use credential_repository::CredentialRepository;
pub use challenge_repository::ChallengeRepository;
