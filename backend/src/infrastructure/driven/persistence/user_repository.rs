use axum::async_trait;
use sqlx::PgPool;
use crate::application::ports::UserRepository;
use crate::domain::{User, UserId, Email, DisplayName, UserRole, UserStatus};

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, String> {
        let result: Option<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id::text, email, display_name, role::text, status::text 
             FROM users 
             WHERE email = $1 AND status = 'active'"
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(result.and_then(|(id, email, display_name, role, status)| {
            let user_id = UserId::from_uuid(id.parse().ok()?);
            let email = Email::new(email).ok()?;
            let display_name = DisplayName::new(display_name).ok()?;
            let role = match role.as_str() {
                "super_admin" => UserRole::SuperAdmin,
                "owner" => UserRole::Owner,
                "client" => UserRole::Client,
                _ => UserRole::Client,
            };
            let status = match status.as_str() {
                "active" => UserStatus::Active,
                "suspended" => UserStatus::Suspended,
                "deleted" => UserStatus::Deleted,
                _ => UserStatus::Active,
            };
            Some(User::from_persistence(user_id, email, display_name, role, status))
        }))
    }
    
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, String> {
        let result: Option<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id::text, email, display_name, role::text, status::text 
             FROM users 
             WHERE id = $1"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(result.and_then(|(id, email, display_name, role, status)| {
            let user_id = UserId::from_uuid(id.parse().ok()?);
            let email = Email::new(email).ok()?;
            let display_name = DisplayName::new(display_name).ok()?;
            let role = match role.as_str() {
                "super_admin" => UserRole::SuperAdmin,
                "owner" => UserRole::Owner,
                "client" => UserRole::Client,
                _ => UserRole::Client,
            };
            let status = match status.as_str() {
                "active" => UserStatus::Active,
                "suspended" => UserStatus::Suspended,
                "deleted" => UserStatus::Deleted,
                _ => UserStatus::Active,
            };
            Some(User::from_persistence(user_id, email, display_name, role, status))
        }))
    }
    
    async fn save(&self, user: &User) -> Result<(), String> {
        let role_str = match user.role() {
            UserRole::SuperAdmin => "super_admin",
            UserRole::Owner => "owner",
            UserRole::Client => "client",
        };
        
        let status_str = match user.status() {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        };
        
        sqlx::query(
            "INSERT INTO users (id, email, display_name, role, status) 
             VALUES ($1, $2, $3, $4::user_role, $5::user_status)"
        )
        .bind(user.id().to_string())
        .bind(user.email().as_str())
        .bind(user.display_name().as_str())
        .bind(role_str)
        .bind(status_str)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;
        
        Ok(())
    }
    
    async fn count_super_admins(&self) -> Result<i64, String> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE role = 'super_admin' AND status = 'active'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(count)
    }
    
    async fn count_users(&self) -> Result<i64, String> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(count)
    }
}
