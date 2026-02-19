use crate::domain::value_objects::UserId;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: uuid::Uuid,
    pub user_id: UserId,
    pub acting_as_owner_id: Option<UserId>,
    pub active_role: String,
    pub app_id: String,
    pub display_number: Option<i32>,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub terminated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Session {
    pub fn new(
        user_id: UserId,
        acting_as_owner_id: Option<UserId>,
        active_role: String,
        app_id: String,
        display_number: Option<i32>,
        session_timeout_secs: u64,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            user_id,
            acting_as_owner_id,
            active_role,
            app_id,
            display_number,
            state: "initializing".to_string(),
            created_at: now,
            expires_at: now + chrono::Duration::seconds(session_timeout_secs as i64),
            terminated_at: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.terminated_at.is_none()
            && self.state != "terminated"
            && self.expires_at > chrono::Utc::now()
    }
}
