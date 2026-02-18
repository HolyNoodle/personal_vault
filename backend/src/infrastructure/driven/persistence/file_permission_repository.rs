use std::sync::Arc;
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use crate::application::ports::file_permission_repository::FilePermissionRepository;
use crate::domain::entities::file_permission::FilePermission;
use crate::domain::entities::invitation::AccessLevel;
use crate::domain::value_objects::UserId;
use crate::infrastructure::driven::persistence::db_types::DbFilePermission;

pub struct SqliteFilePermissionRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl SqliteFilePermissionRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self { pool }
    }
}

fn db_to_file_permission(row: DbFilePermission) -> Result<FilePermission, String> {
    let id = uuid::Uuid::parse_str(&row.id).map_err(|e| format!("Invalid id: {e}"))?;
    let owner_uuid = uuid::Uuid::parse_str(&row.owner_id).map_err(|e| format!("Invalid owner_id: {e}"))?;
    let client_uuid = uuid::Uuid::parse_str(&row.client_id).map_err(|e| format!("Invalid client_id: {e}"))?;
    let access: Vec<AccessLevel> = serde_json::from_str(&row.access)
        .map_err(|e| format!("Failed to parse access: {e}"))?;
    let granted_at = row
        .granted_at
        .parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap_or_else(|_| chrono::Utc::now());
    let expires_at = row
        .expires_at
        .as_deref()
        .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
        .transpose()
        .map_err(|e| format!("Invalid expires_at: {e}"))?;
    let revoked_at = row
        .revoked_at
        .as_deref()
        .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
        .transpose()
        .map_err(|e| format!("Invalid revoked_at: {e}"))?;

    Ok(FilePermission {
        id,
        owner_id: UserId::from_uuid(owner_uuid),
        client_id: UserId::from_uuid(client_uuid),
        path: row.path,
        access,
        granted_at,
        expires_at,
        revoked_at,
    })
}

#[async_trait]
impl FilePermissionRepository for SqliteFilePermissionRepository {
    async fn save(&self, permission: &FilePermission) -> Result<(), String> {
        let id = permission.id.to_string();
        let owner_id = permission.owner_id.to_string();
        let client_id = permission.client_id.to_string();
        let path = permission.path.clone();
        let access = serde_json::to_string(&permission.access)
            .map_err(|e| format!("Failed to serialize access: {e}"))?;
        let granted_at = permission.granted_at.to_rfc3339();
        let expires_at = permission
            .expires_at
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339());
        let revoked_at = permission
            .revoked_at
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339());
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query(
                "INSERT INTO file_permissions (id, owner_id, client_id, path, access, granted_at, expires_at, revoked_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
            )
            .bind::<diesel::sql_types::Text, _>(&id)
            .bind::<diesel::sql_types::Text, _>(&owner_id)
            .bind::<diesel::sql_types::Text, _>(&client_id)
            .bind::<diesel::sql_types::Text, _>(&path)
            .bind::<diesel::sql_types::Text, _>(&access)
            .bind::<diesel::sql_types::Text, _>(&granted_at)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&expires_at)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&revoked_at)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save file permission: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_active_for_client(&self, client_id: &UserId) -> Result<Vec<FilePermission>, String> {
        let client_id_str = client_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<FilePermission>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbFilePermission> = diesel::sql_query(
                "SELECT id, owner_id, client_id, path, access, granted_at, expires_at, revoked_at \
                 FROM file_permissions \
                 WHERE client_id = ?1 AND revoked_at IS NULL \
                 AND (expires_at IS NULL OR expires_at > datetime('now'))"
            )
            .bind::<diesel::sql_types::Text, _>(&client_id_str)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().map(db_to_file_permission).collect()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_active_by_owner(&self, owner_id: &UserId) -> Result<Vec<FilePermission>, String> {
        let owner_id_str = owner_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<FilePermission>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbFilePermission> = diesel::sql_query(
                "SELECT id, owner_id, client_id, path, access, granted_at, expires_at, revoked_at \
                 FROM file_permissions \
                 WHERE owner_id = ?1 AND revoked_at IS NULL"
            )
            .bind::<diesel::sql_types::Text, _>(&owner_id_str)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().map(db_to_file_permission).collect()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_by_owner_client(
        &self,
        owner_id: &UserId,
        client_id: &UserId,
    ) -> Result<Vec<FilePermission>, String> {
        let owner_id_str = owner_id.to_string();
        let client_id_str = client_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<FilePermission>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbFilePermission> = diesel::sql_query(
                "SELECT id, owner_id, client_id, path, access, granted_at, expires_at, revoked_at \
                 FROM file_permissions \
                 WHERE owner_id = ?1 AND client_id = ?2"
            )
            .bind::<diesel::sql_types::Text, _>(&owner_id_str)
            .bind::<diesel::sql_types::Text, _>(&client_id_str)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().map(db_to_file_permission).collect()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn revoke(&self, id: &uuid::Uuid) -> Result<(), String> {
        let id_str = id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query(
                "UPDATE file_permissions SET revoked_at = datetime('now') WHERE id = ?1"
            )
            .bind::<diesel::sql_types::Text, _>(&id_str)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to revoke permission: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }
}
