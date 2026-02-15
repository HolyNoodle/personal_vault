use crate::domain::aggregates::VideoSession;
// ...existing code...
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
// ...existing code...
// Removed import for deleted VideoSessionRepository trait

/// In-memory implementation of VideoSessionRepository
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

// Removed trait implementation for deleted VideoSessionRepository methods
