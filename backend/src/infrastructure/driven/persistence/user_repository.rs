use std::sync::Arc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use async_trait::async_trait;
use crate::application::ports::user_repository::UserRepository;
use crate::domain::User;
use crate::infrastructure::driven::persistence::schema::users;
use crate::infrastructure::driven::persistence::db_types::{DbUser, NewDbUser};

pub struct SqliteUserRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl SqliteUserRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn count_super_admins(&self) -> Result<u64, String> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let count: i64 = users::table
                .filter(users::roles.like("%\"super_admin\"%"))
                .count()
                .get_result(&mut conn)
                .map_err(|e| e.to_string())?;
            Ok(count as u64)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn find_by_email(&self, email: &crate::domain::Email) -> Result<Option<crate::domain::User>, String> {
        let email_str = email.as_str().to_string();
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let result = users::table
                .filter(users::email.eq(&email_str))
                .first::<DbUser>(&mut conn)
                .optional()
                .map_err(|e| e.to_string())?;

            if let Some(db_user) = result {
                let id = uuid::Uuid::parse_str(&db_user.id)
                    .map_err(|e| format!("Invalid UUID in DB: {}", e))?;
                let roles_strs: Vec<String> = serde_json::from_str(&db_user.roles)
                    .unwrap_or_default();
                let roles = roles_strs
                    .iter()
                    .filter_map(|r| match r.as_str() {
                        "super_admin" => Some(crate::domain::UserRole::SuperAdmin),
                        "owner" => Some(crate::domain::UserRole::Owner),
                        "client" => Some(crate::domain::UserRole::Client),
                        _ => None,
                    })
                    .collect();
                let status = match db_user.status.as_str() {
                    "suspended" => crate::domain::UserStatus::Suspended,
                    "deleted" => crate::domain::UserStatus::Deleted,
                    _ => crate::domain::UserStatus::Active,
                };
                let user = crate::domain::User::from_persistence(
                    crate::domain::UserId::from_uuid(id),
                    crate::domain::Email::new(db_user.email).map_err(|e| e.to_string())?,
                    crate::domain::DisplayName::new(db_user.display_name).map_err(|e| e.to_string())?,
                    roles,
                    status,
                );
                Ok(Some(user))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn save(&self, user: &User) -> Result<(), String> {
        let id = user.id().to_string();
        let email = user.email().as_str().to_string();
        let display_name = user.display_name().as_str().to_string();
        let roles = serde_json::to_string(
            &user.roles().iter().map(|r| r.as_db_str()).collect::<Vec<_>>()
        ).map_err(|e| e.to_string())?;
        let status = user.status().as_db_str().to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let new_user = NewDbUser { id, email, display_name, roles, status };
            diesel::insert_into(users::table)
                .values(&new_user)
                .execute(&mut conn)
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
