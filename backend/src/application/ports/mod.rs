// Application ports - Driven ports (output ports implemented by infrastructure)

pub mod user_repository;
pub mod credential_repository;
pub mod challenge_repository;
pub mod application_ports;

// Removed pub use for UserRepository
pub use credential_repository::CredentialRepository;
pub use challenge_repository::ChallengeRepository;

// Legacy video ports (being phased out)
// Removed pub use for deleted video port traits

// New application-centric ports
    // Removed pub use for deleted application port traits and structs
// Removed stray closing delimiter
