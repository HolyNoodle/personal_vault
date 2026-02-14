use crate::domain::value_objects::*;

#[derive(Debug, Clone)]
pub struct UserRegistered {
    pub user_id: UserId,
    pub email: Email,
    pub role: UserRole,
}
