// Removed unused imports
use sqlx::{PgPool, Row};
use crate::infrastructure::driven::persistence::db_types::{DbUserRole, DbUserStatus};

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

use async_trait::async_trait;
use crate::application::ports::user_repository::UserRepository;
use crate::domain::User;

#[async_trait]
impl UserRepository for PostgresUserRepository {
            async fn count_super_admins(&self) -> Result<u64, String> {
                let row = sqlx::query("SELECT COUNT(*) FROM users WHERE role = 'super_admin'")
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;
                let count: i64 = row.try_get(0).map_err(|e| e.to_string())?;
                Ok(count as u64)
            }
        async fn find_by_email(&self, email: &crate::domain::Email) -> Result<Option<crate::domain::User>, String> {
            let row = sqlx::query(
                "SELECT id, email, display_name, role, status FROM users WHERE email = $1"
            )
            .bind(email.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            if let Some(row) = row {
                let id: uuid::Uuid = row.try_get("id").map_err(|e| e.to_string())?;
                let email_str: String = row.try_get("email").map_err(|e| e.to_string())?;
                let display_name: String = row.try_get("display_name").map_err(|e| e.to_string())?;
                let role: DbUserRole = row.try_get("role").map_err(|e| e.to_string())?;
                let status: DbUserStatus = row.try_get("status").map_err(|e| e.to_string())?;
                let user = crate::domain::User::from_persistence(
                    crate::domain::UserId::from_uuid(id),
                    crate::domain::Email::new(email_str).map_err(|e| e.to_string())?,
                    crate::domain::DisplayName::new(display_name).map_err(|e| e.to_string())?,
                    role.to_domain(),
                    status.to_domain(),
                );
                Ok(Some(user))
            } else {
                Ok(None)
            }
        }
    async fn save(&self, user: &User) -> Result<(), String> {
        let query = "INSERT INTO users (id, email, display_name, role, status) VALUES ($1, $2, $3, $4::user_role, $5::user_status)";
        sqlx::query(query)
            .bind(user.id().as_uuid())
            .bind(user.email().as_str())
            .bind(user.display_name().as_str())
            .bind(user.role().as_db_str())
            .bind(user.status().as_db_str())
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
