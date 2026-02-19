use std::sync::Arc;
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use crate::application::ports::session_repository::SessionRepository;
use crate::domain::entities::session::Session;
use crate::domain::value_objects::UserId;
use crate::infrastructure::driven::persistence::db_types::DbSession;

pub struct SqliteSessionRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl SqliteSessionRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self { pool }
    }
}

fn db_to_session(row: DbSession) -> Result<Session, String> {
    let id = uuid::Uuid::parse_str(&row.id).map_err(|e| format!("Invalid session id: {e}"))?;
    let user_uuid = uuid::Uuid::parse_str(&row.user_id).map_err(|e| format!("Invalid user_id: {e}"))?;
    let acting_as_owner_id = row.acting_as_owner_id
        .as_deref()
        .map(|s| uuid::Uuid::parse_str(s).map(UserId::from_uuid))
        .transpose()
        .map_err(|e| format!("Invalid acting_as_owner_id: {e}"))?;
    let created_at = row.created_at
        .parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap_or_else(|_| chrono::Utc::now());
    let expires_at = row.expires_at
        .parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap_or_else(|_| chrono::Utc::now());
    let terminated_at = row.terminated_at
        .as_deref()
        .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
        .transpose()
        .map_err(|e| format!("Invalid terminated_at: {e}"))?;

    Ok(Session {
        id,
        user_id: UserId::from_uuid(user_uuid),
        acting_as_owner_id,
        active_role: row.active_role,
        app_id: row.app_id,
        display_number: row.display_number,
        state: row.state,
        created_at,
        expires_at,
        terminated_at,
    })
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save(&self, session: &Session) -> Result<(), String> {
        let id = session.id.to_string();
        let user_id = session.user_id.to_string();
        let acting_as_owner_id = session.acting_as_owner_id.as_ref().map(|u| u.to_string());
        let active_role = session.active_role.clone();
        let app_id = session.app_id.clone();
        let display_number = session.display_number;
        let state = session.state.clone();
        let created_at = session.created_at.to_rfc3339();
        let expires_at = session.expires_at.to_rfc3339();
        let terminated_at = session.terminated_at.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339());
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query(
                "INSERT INTO sessions (id, user_id, acting_as_owner_id, active_role, app_id, display_number, state, created_at, expires_at, terminated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                 ON CONFLICT(id) DO UPDATE SET state=excluded.state, terminated_at=excluded.terminated_at"
            )
            .bind::<diesel::sql_types::Text, _>(&id)
            .bind::<diesel::sql_types::Text, _>(&user_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&acting_as_owner_id)
            .bind::<diesel::sql_types::Text, _>(&active_role)
            .bind::<diesel::sql_types::Text, _>(&app_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(&display_number)
            .bind::<diesel::sql_types::Text, _>(&state)
            .bind::<diesel::sql_types::Text, _>(&created_at)
            .bind::<diesel::sql_types::Text, _>(&expires_at)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&terminated_at)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save session: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_by_id(&self, id: &uuid::Uuid) -> Result<Option<Session>, String> {
        let id_str = id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Session>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbSession> = diesel::sql_query(
                "SELECT id, user_id, acting_as_owner_id, active_role, app_id, display_number, state, created_at, expires_at, terminated_at \
                 FROM sessions WHERE id = ?1"
            )
            .bind::<diesel::sql_types::Text, _>(&id_str)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().next().map(db_to_session).transpose()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_active_by_user(&self, user_id: &UserId) -> Result<Vec<Session>, String> {
        let user_id_str = user_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Session>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbSession> = diesel::sql_query(
                "SELECT id, user_id, acting_as_owner_id, active_role, app_id, display_number, state, created_at, expires_at, terminated_at \
                 FROM sessions WHERE user_id = ?1 AND state != 'terminated' AND terminated_at IS NULL \
                 AND expires_at > datetime('now')"
            )
            .bind::<diesel::sql_types::Text, _>(&user_id_str)
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().map(db_to_session).collect()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn update_state(&self, id: &uuid::Uuid, state: &str) -> Result<(), String> {
        let id_str = id.to_string();
        let state = state.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query("UPDATE sessions SET state = ?1 WHERE id = ?2")
                .bind::<diesel::sql_types::Text, _>(&state)
                .bind::<diesel::sql_types::Text, _>(&id_str)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update session state: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn terminate(&self, id: &uuid::Uuid) -> Result<(), String> {
        let id_str = id.to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query(
                "UPDATE sessions SET state = 'terminated', terminated_at = ?1 WHERE id = ?2"
            )
            .bind::<diesel::sql_types::Text, _>(&now)
            .bind::<diesel::sql_types::Text, _>(&id_str)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to terminate session: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }

    async fn find_expired(&self) -> Result<Vec<Session>, String> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Session>, String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbSession> = diesel::sql_query(
                "SELECT id, user_id, acting_as_owner_id, active_role, app_id, display_number, state, created_at, expires_at, terminated_at \
                 FROM sessions WHERE state != 'terminated' AND terminated_at IS NULL \
                 AND expires_at <= datetime('now')"
            )
            .load(&mut conn)
            .map_err(|e| format!("Database error: {e}"))?;

            rows.into_iter().map(db_to_session).collect()
        })
        .await
        .map_err(|e: tokio::task::JoinError| e.to_string())?
    }
}
