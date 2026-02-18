use async_trait::async_trait;
use crate::domain::entities::invitation::Invitation;

#[async_trait]
pub trait InvitationRepository: Send + Sync {
    async fn save(&self, invitation: &Invitation) -> Result<(), String>;
    async fn find_by_token(&self, token: &str) -> Result<Option<Invitation>, String>;
    async fn find_by_owner(&self, owner_id: &crate::domain::value_objects::UserId) -> Result<Vec<Invitation>, String>;
    async fn update_status(&self, id: &uuid::Uuid, status: &str) -> Result<(), String>;
}
