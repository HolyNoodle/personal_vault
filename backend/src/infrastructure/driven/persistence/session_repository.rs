// ...existing code...
use crate::domain::aggregates::ApplicationSession;
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

// Removed trait implementation for deleted ApplicationSessionRepository
