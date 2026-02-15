// Driven port - User repository (output port)

use axum::async_trait;
// ...existing code...

#[async_trait]
pub trait UserRepository: Send + Sync {
    // Removed all unused methods from UserRepository trait
}
