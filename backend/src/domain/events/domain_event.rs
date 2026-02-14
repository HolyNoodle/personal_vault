use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DomainEvent<T> {
    pub event_id: uuid::Uuid,
    pub occurred_at: DateTime<Utc>,
    pub data: T,
}

impl<T> DomainEvent<T> {
    pub fn new(data: T) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4(),
            occurred_at: Utc::now(),
            data,
        }
    }
}
