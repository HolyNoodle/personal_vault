use std::sync::Arc;
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use crate::application::ports::invitation_repository::InvitationRepository;
use crate::domain::entities::invitation::{Invitation, InvitationStatus, GrantedPath};
use crate::domain::value_objects::{Email, UserId};
use crate::infrastructure::driven::persistence::db_types::DbInvitation;

pub struct SqliteInvitationRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl SqliteInvitationRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self { pool }
    }
}

fn db_to_invitation(row: DbInvitation) -> Result<Invitation, String> {
    let id = uuid::Uuid::parse_str(&row.id).map_err(|e| format!("Invalid invitation id: {e}"))?;
    let owner_uuid = uuid::Uuid::parse_str(&row.owner_id).map_err(|e| format!("Invalid owner_id: {e}"))?;
    let owner_id = UserId::from_uuid(owner_uuid);
    let invitee_email = Email::new(row.invitee_email).map_err(|e| e.to_string())?;
    let granted_paths: Vec<GrantedPath> = serde_json::from_str(&row.granted_paths)
        .map_err(|e| format!("Failed to parse granted_paths: {e}"))?;
    let status = match row.status.as_str() {
        "Accepted" => InvitationStatus::Accepted,
        "Revoked" => InvitationStatus::Revoked,
        "Expired" => InvitationStatus::Expired,
        _ => InvitationStatus::Pending,
    };
    let expires_at = row
        .expires_at
        .as_deref()
        .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
        .transpose()
        .map_err(|e| format!("Invalid expires_at: {e}"))?;
    let created_at = row
        .created_at
        .parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(Invitation {
        id,
        owner_id,
        invitee_email,
        token: row.token,
        granted_paths,
        status,
        expires_at,
        created_at,
    })
}

#[async_trait]
impl InvitationRepository for SqliteInvitationRepository {
    async fn save(&self, invitation: &Invitation) -> Result<(), String> {
        let id = invitation.id.to_string();
        let owner_id = invitation.owner_id.to_string();
        let invitee_email = invitation.invitee_email.as_str().to_string();
        let token = invitation.token.clone();
        let granted_paths = serde_json::to_string(&invitation.granted_paths)
            .map_err(|e| format!("Failed to serialize granted_paths: {e}"))?;
        let status = format!("{:?}", invitation.status);
        let expires_at = invitation.expires_at.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339());
        let created_at = invitation.created_at.to_rfc3339();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query(
                "INSERT INTO invitations (id, owner_id, invitee_email, token, granted_paths, status, expires_at, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT(id) DO UPDATE SET status=excluded.status"
            )
            .bind::<diesel::sql_types::Text, _>(&id)
            .bind::<diesel::sql_types::Text, _>(&owner_id)
            .bind::<diesel::sql_types::Text, _>(&invitee_email)
            .bind::<diesel::sql_types::Text, _>(&token)
            .bind::<diesel::sql_types::Text, _>(&granted_paths)
            .bind::<diesel::sql_types::Text, _>(&status)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&expires_at)
            .bind::<diesel::sql_types::Text, _>(&created_at)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save invitation: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<Invitation>, String> {
        let token = token.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Invitation>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbInvitation> = diesel::sql_query(
                "SELECT id, owner_id, invitee_email, token, granted_paths, status, expires_at, created_at \
                 FROM invitations WHERE token = ?1"
            )
            .bind::<diesel::sql_types::Text, _>(&token)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter()
                .next()
                .map(db_to_invitation)
                .transpose()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_by_owner(&self, owner_id: &UserId) -> Result<Vec<Invitation>, String> {
        let owner_id_str = owner_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Invitation>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbInvitation> = diesel::sql_query(
                "SELECT id, owner_id, invitee_email, token, granted_paths, status, expires_at, created_at \
                 FROM invitations WHERE owner_id = ?1"
            )
            .bind::<diesel::sql_types::Text, _>(&owner_id_str)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().map(db_to_invitation).collect()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn update_status(&self, id: &uuid::Uuid, status: &str) -> Result<(), String> {
        let id_str = id.to_string();
        let status = status.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query("UPDATE invitations SET status = ?1 WHERE id = ?2")
                .bind::<diesel::sql_types::Text, _>(&status)
                .bind::<diesel::sql_types::Text, _>(&id_str)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update invitation status: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
