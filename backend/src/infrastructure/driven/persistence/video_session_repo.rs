use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::domain::aggregates::{VideoSession, VideoSessionId};
use crate::application::ports::VideoSessionRepository;

/// In-memory implementation of VideoSessionRepository (for POC)
/// In production, this would use PostgreSQL
pub struct InMemoryVideoSessionRepository {
    sessions: Arc<RwLock<HashMap<String, VideoSession>>>,
}

impl InMemoryVideoSessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl VideoSessionRepository for InMemoryVideoSessionRepository {
    async fn save(&self, session: &VideoSession) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.to_string(), session.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &VideoSessionId) -> Result<Option<VideoSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id.as_str()).cloned())
    }

    async fn find_by_user_id(&self, user_id: &str) -> Result<Vec<VideoSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &VideoSessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id.as_str());
        Ok(())
    }
}
