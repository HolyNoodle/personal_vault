use crate::domain::value_objects::UserId;

#[derive(Debug, Clone)]
pub struct UserActivated {
    pub user_id: UserId,
}
