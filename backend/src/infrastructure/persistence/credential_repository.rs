use axum::async_trait;
use sqlx::PgPool;
use crate::application::ports::{CredentialRepository, WebAuthnCredential, UserId};

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
    async fn find_by_user_id(&self, user_id: &UserId) -> Result<Vec<WebAuthnCredential>, String> {
        let rows: Vec<(Vec<u8>, Vec<u8>, i64)> = sqlx::query_as(
            "SELECT credential_id, public_key, sign_count 
             FROM webauthn_credentials 
             WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        let credentials = rows
            .into_iter()
            .filter_map(|(credential_id, public_key, sign_count)| {
                std::str::from_utf8(&public_key)
                    .ok()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .map(|passkey| WebAuthnCredential {
                        user_id: *user_id,
                        credential_id,
                        passkey,
                        sign_count,
                    })
            })
            .collect();
        
        Ok(credentials)
    }
    
    async fn create(&self, credential: &WebAuthnCredential) -> Result<(), String> {
        let passkey_json = serde_json::to_string(&credential.passkey)
            .map_err(|e| format!("Failed to serialize passkey: {}", e))?;
        
        let transports_json = serde_json::json!(
            credential.passkey.transports()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<_>>()
        );
        
        sqlx::query(
            "INSERT INTO webauthn_credentials (user_id, credential_id, public_key, sign_count, transports) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&credential.user_id)
        .bind(&credential.credential_id)
        .bind(passkey_json.as_bytes())
        .bind(credential.sign_count)
        .bind(&transports_json)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create credential: {}", e))?;
        
        Ok(())
    }
    
    async fn update_sign_count(&self, user_id: &UserId, credential_id: &[u8], sign_count: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE webauthn_credentials 
             SET sign_count = $1, updated_at = NOW() 
             WHERE user_id = $2 AND credential_id = $3"
        )
        .bind(sign_count)
        .bind(user_id)
        .bind(credential_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sign count: {}", e))?;
        
        Ok(())
    }
}
