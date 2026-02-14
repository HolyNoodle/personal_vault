// Domain Entities - Objects with identity and lifecycle

use super::value_objects::*;
use webauthn_rs::prelude::Passkey;

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    display_name: DisplayName,
    role: UserRole,
    status: UserStatus,
}

impl User {
    pub fn new(email: Email, display_name: DisplayName, role: UserRole) -> Self {
        Self {
            id: UserId::new(),
            email,
            display_name,
            role,
            status: UserStatus::Active,
        }
    }
    
    pub fn from_persistence(
        id: UserId,
        email: Email,
        display_name: DisplayName,
        role: UserRole,
        status: UserStatus,
    ) -> Self {
        Self {
            id,
            email,
            display_name,
            role,
            status,
        }
    }
    
    pub fn id(&self) -> &UserId {
        &self.id
    }
    
    pub fn email(&self) -> &Email {
        &self.email
    }
    
    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }
    
    pub fn role(&self) -> UserRole {
        self.role
    }
    
    pub fn status(&self) -> UserStatus {
        self.status
    }
    
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }
    
    pub fn suspend(&mut self) {
        self.status = UserStatus::Suspended;
    }
    
    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
    }
}

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
            credential_id: passkey.cred_id().0.to_vec(),
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
    
    pub fn update_sign_count(&mut self, new_count: u32) {
        self.sign_count = new_count;
    }
}
