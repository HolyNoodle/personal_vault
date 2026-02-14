use axum::async_trait;
use redis::AsyncCommands;
use crate::application::ports::ChallengeRepository;

pub struct RedisChallengeRepository {
    client: redis::Client,
}

impl RedisChallengeRepository {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ChallengeRepository for RedisChallengeRepository {
    async fn save_registration_challenge(&self, challenge_id: &str, state: &str, ttl_seconds: u64) -> Result<(), String> {
        let mut conn = self.client.get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))?;
        
        let key = format!("webauthn:challenge:{}", challenge_id);
        conn.set_ex(&key, state, ttl_seconds as usize)
            .await
            .map_err(|e| format!("Failed to save challenge: {}", e))?;
        
        Ok(())
    }
    
    async fn get_and_delete_registration_challenge(&self, challenge_id: &str) -> Result<String, String> {
        let mut conn = self.client.get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))?;
        
        let key = format!("webauthn:challenge:{}", challenge_id);
        conn.get_del(&key)
            .await
            .map_err(|_| "Invalid or expired challenge".to_string())
    }
    
    async fn save_auth_challenge(&self, challenge_id: &str, state: &str, ttl_seconds: u64) -> Result<(), String> {
        let mut conn = self.client.get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))?;
        
        let key = format!("webauthn:auth:{}", challenge_id);
        conn.set_ex(&key, state, ttl_seconds as usize)
            .await
            .map_err(|e| format!("Failed to save auth challenge: {}", e))?;
        
        Ok(())
    }
    
    async fn get_and_delete_auth_challenge(&self, challenge_id: &str) -> Result<String, String> {
        let mut conn = self.client.get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))?;
        
        let key = format!("webauthn:auth:{}", challenge_id);
        conn.get_del(&key)
            .await
            .map_err(|_| "Invalid or expired challenge".to_string())
    }
}
