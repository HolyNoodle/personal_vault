use axum::async_trait;
use sqlx::PgPool;
use crate::application::ports::{User, UserRepository, UserId, UserRole, UserStatus};

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
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
        let result: Option<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id::text, email, display_name, role::text, status::text 
             FROM users 
             WHERE email = $1 AND status = 'active'"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(result.map(|(id, email, display_name, role, status)| User {
            id: id.parse().unwrap(),
            email,
            display_name,
            role: match role.as_str() {
                "super_admin" => UserRole::SuperAdmin,
                "owner" => UserRole::Owner,
                "client" => UserRole::Client,
                _ => UserRole::Client,
            },
            status: match status.as_str() {
                "active" => UserStatus::Active,
                "suspended" => UserStatus::Suspended,
                "deleted" => UserStatus::Deleted,
                _ => UserStatus::Active,
            },
        }))
    }
    
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, String> {
        let result: Option<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id::text, email, display_name, role::text, status::text 
             FROM users 
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(result.map(|(id, email, display_name, role, status)| User {
            id: id.parse().unwrap(),
            email,
            display_name,
            role: match role.as_str() {
                "super_admin" => UserRole::SuperAdmin,
                "owner" => UserRole::Owner,
                "client" => UserRole::Client,
                _ => UserRole::Client,
            },
            status: match status.as_str() {
                "active" => UserStatus::Active,
                "suspended" => UserStatus::Suspended,
                "deleted" => UserStatus::Deleted,
                _ => UserStatus::Active,
            },
        }))
    }
    
    async fn create(&self, user: &User) -> Result<(), String> {
        let role_str = match user.role {
            UserRole::SuperAdmin => "super_admin",
            UserRole::Owner => "owner",
            UserRole::Client => "client",
        };
        
        let status_str = match user.status {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        };
        
        sqlx::query(
            "INSERT INTO users (id, email, display_name, role, status) 
             VALUES ($1, $2, $3, $4::user_role, $5::user_status)"
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.display_name)
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
}
