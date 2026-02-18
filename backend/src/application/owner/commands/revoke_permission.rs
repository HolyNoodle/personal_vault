use crate::application::ports::file_permission_repository::FilePermissionRepository;
use uuid::Uuid;

pub async fn execute<R: FilePermissionRepository + ?Sized>(
    repo: &R,
    permission_id: &Uuid,
) -> Result<(), String> {
    repo.revoke(permission_id).await
}
