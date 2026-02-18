use crate::application::ports::file_permission_repository::FilePermissionRepository;
use crate::domain::value_objects::UserId;

pub async fn execute<R: FilePermissionRepository + ?Sized>(
    repo: &R,
    client_id: &UserId,
) -> Result<Vec<crate::domain::entities::file_permission::FilePermission>, String> {
    repo.find_active_for_client(client_id).await
}
