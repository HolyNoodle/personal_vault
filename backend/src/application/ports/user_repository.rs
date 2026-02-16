// Driven port - User repository (output port)

use axum::async_trait;
// ...existing code...

#[async_trait]
pub trait UserRepository: Send + Sync {
        async fn count_super_admins(&self) -> Result<u64, String>;
    async fn save(&self, user: &crate::domain::User) -> Result<(), String>;
    async fn find_by_email(&self, email: &crate::domain::Email) -> Result<Option<crate::domain::User>, String>;
}
