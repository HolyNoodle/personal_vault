use async_trait::async_trait;
use crate::domain::entities::session::Session;
use crate::domain::value_objects::UserId;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save(&self, session: &Session) -> Result<(), String>;
    async fn find_by_id(&self, id: &uuid::Uuid) -> Result<Option<Session>, String>;
    async fn find_active_by_user(&self, user_id: &UserId) -> Result<Vec<Session>, String>;
    async fn update_state(&self, id: &uuid::Uuid, state: &str) -> Result<(), String>;
    async fn terminate(&self, id: &uuid::Uuid) -> Result<(), String>;
    async fn find_expired(&self) -> Result<Vec<Session>, String>;
}
