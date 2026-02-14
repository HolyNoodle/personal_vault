use axum::async_trait;
use sqlx::PgPool;
use crate::application::ports::CredentialRepository;
use crate::domain::{Credential, UserId};

pub struct PostgresCredentialRepository {
    pool: PgPool,
}

impl PostgresCredentialRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CredentialRepository for PostgresCredentialRepository {
    async fn find_by_user_id(&self, user_id: &UserId) -> Result<Vec<Credential>, String> {
        let uuid_str = user_id.to_string();
        let uuid = uuid::Uuid::parse_str(&uuid_str)
            .map_err(|e| format!("Invalid UUID: {}", e))?;
        
        let rows: Vec<(Vec<u8>, Vec<u8>, i64)> = sqlx::query_as(
            "SELECT credential_id, public_key, sign_count 
             FROM webauthn_credentials 
             WHERE user_id = $1"
        )
        .bind(uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        let credentials = rows
            .into_iter()
            .filter_map(|(credential_id, public_key, sign_count)| {
                std::str::from_utf8(&public_key)
                    .ok()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .map(|passkey| Credential::from_persistence(
                        user_id.clone(),
                        credential_id,
                        passkey,
                        sign_count as u32,
                    ))
            })
            .collect();
        
        Ok(credentials)
    }
    
    async fn save(&self, credential: &Credential) -> Result<(), String> {
        let passkey_json = serde_json::to_string(credential.passkey())
            .map_err(|e| format!("Failed to serialize passkey: {}", e))?;
        
        let uuid_str = credential.user_id().to_string();
        let uuid = uuid::Uuid::parse_str(&uuid_str)
            .map_err(|e| format!("Invalid UUID: {}", e))?;
        
        sqlx::query(
            "INSERT INTO webauthn_credentials (user_id, credential_id, public_key, sign_count) 
             VALUES ($1, $2, $3, $4)"
        )
        .bind(uuid)
        .bind(credential.credential_id())
        .bind(passkey_json.as_bytes())
        .bind(credential.sign_count() as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create credential: {}", e))?;
        
        Ok(())
    }
    
    async fn update_sign_count(&self, user_id: &UserId, credential_id: &[u8], sign_count: u32) -> Result<(), String> {
        let uuid_str = user_id.to_string();
        let uuid = uuid::Uuid::parse_str(&uuid_str)
            .map_err(|e| format!("Invalid UUID: {}", e))?;
        
        sqlx::query(
            "UPDATE webauthn_credentials 
             SET sign_count = $1, updated_at = NOW() 
             WHERE user_id = $2 AND credential_id = $3"
        )
        .bind(sign_count as i64)
        .bind(uuid)
        .bind(credential_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sign count: {}", e))?;
        
        Ok(())
    }
}
