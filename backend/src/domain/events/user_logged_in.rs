use crate::domain::value_objects::*;

#[derive(Debug, Clone)]
pub struct UserLoggedIn {
    pub user_id: UserId,
    pub email: Email,
}
