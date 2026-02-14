use crate::domain::value_objects::UserId;

#[derive(Debug, Clone)]
pub struct CredentialAdded {
    pub user_id: UserId,
    pub credential_id: Vec<u8>,
}
