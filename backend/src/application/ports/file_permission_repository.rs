use async_trait::async_trait;
use crate::domain::entities::file_permission::FilePermission;

#[async_trait]
pub trait FilePermissionRepository: Send + Sync {
    async fn save(&self, permission: &FilePermission) -> Result<(), String>;
    async fn find_active_for_client(&self, client_id: &crate::domain::value_objects::UserId) -> Result<Vec<FilePermission>, String>;
    async fn find_by_owner_client(&self, owner_id: &crate::domain::value_objects::UserId, client_id: &crate::domain::value_objects::UserId) -> Result<Vec<FilePermission>, String>;
    async fn find_active_by_owner(&self, owner_id: &crate::domain::value_objects::UserId) -> Result<Vec<FilePermission>, String>;
    async fn revoke(&self, id: &uuid::Uuid) -> Result<(), String>;
}
