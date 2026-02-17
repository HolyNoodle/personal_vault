// Driven port - Challenge repository (output port)

use async_trait::async_trait;

#[async_trait]
pub trait ChallengeRepository: Send + Sync {
    async fn save_registration_challenge(&self, challenge_id: &str, state: &str, ttl_seconds: u64) -> Result<(), String>;
    async fn get_and_delete_registration_challenge(&self, challenge_id: &str) -> Result<String, String>;
    async fn save_auth_challenge(&self, challenge_id: &str, state: &str, ttl_seconds: u64) -> Result<(), String>;
    async fn get_and_delete_auth_challenge(&self, challenge_id: &str) -> Result<String, String>;
}
