// Driven port - User repository (output port)

use axum::async_trait;
use crate::domain::{User, UserId, Email};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, String>;
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, String>;
    async fn save(&self, user: &User) -> Result<(), String>;
    async fn count_super_admins(&self) -> Result<i64, String>;
    async fn count_users(&self) -> Result<i64, String>;
}
