use crate::domain::value_objects::UserId;

#[derive(Debug, Clone)]
pub struct UserSuspended {
    pub user_id: UserId,
    pub reason: String,
}
