use crate::domain::value_objects::*;
use webauthn_rs::prelude::Passkey;

#[derive(Debug, Clone)]
pub struct Credential {
    user_id: UserId,
    credential_id: Vec<u8>,
    passkey: Passkey,
    sign_count: u32,
}

impl Credential {
    pub fn new(user_id: UserId, passkey: Passkey) -> Self {
        Self {
            user_id,
            credential_id: passkey.cred_id().as_ref().to_vec(),
            passkey,
            sign_count: 0,
        }
    }
    
    pub fn from_persistence(
        user_id: UserId,
        credential_id: Vec<u8>,
        passkey: Passkey,
        sign_count: u32,
    ) -> Self {
        Self {
            user_id,
            credential_id,
            passkey,
            sign_count,
        }
    }
    
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
    
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }
    
    pub fn passkey(&self) -> &Passkey {
        &self.passkey
    }
    
    pub fn sign_count(&self) -> u32 {
        self.sign_count
    }
    
    // Removed unused method update_sign_count
}
