use anyhow::Result;
use crate::domain::aggregates::{ApplicationSession, SessionId};
use crate::application::ports::ApplicationSessionRepository;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory application session repository (for testing/development)
/// In production, this would use PostgreSQL
pub struct InMemorySessionRepository {
    sessions: RwLock<HashMap<String, ApplicationSession>>,
}

impl InMemorySessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApplicationSessionRepository for InMemorySessionRepository {
    async fn save(&self, session: &ApplicationSession) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.id.to_string(), session.clone());
        println!("💾 Saved session: {}", session.id);
        Ok(())
    }

    async fn find_by_id(&self, id: &SessionId) -> Result<Option<ApplicationSession>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(id.as_str()).cloned())
    }

    async fn find_by_user_id(&self, user_id: &str) -> Result<Vec<ApplicationSession>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn find_active_sessions(&self) -> Result<Vec<ApplicationSession>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions
            .values()
            .filter(|s| s.is_active())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(id.as_str());
        println!("🗑️  Deleted session: {}", id);
        Ok(())
    }

    async fn update_activity(&self, id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(id.as_str()) {
            session.update_activity();
            println!("⏰ Updated activity for session: {}", id);
        }
        Ok(())
    }
}
