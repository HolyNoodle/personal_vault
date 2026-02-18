use crate::application::ports::file_permission_repository::FilePermissionRepository;
use crate::domain::value_objects::UserId;

pub async fn execute<R: FilePermissionRepository + ?Sized>(
    repo: &R,
    owner_id: &UserId,
    client_id: Option<&UserId>,
) -> Result<Vec<crate::domain::entities::file_permission::FilePermission>, String> {
    if let Some(cid) = client_id {
        repo.find_by_owner_client(owner_id, cid).await
    } else {
        repo.find_active_by_owner(owner_id).await
    }
}
