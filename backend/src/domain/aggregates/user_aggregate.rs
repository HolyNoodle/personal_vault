use crate::domain::entities::{User, Credential};
use crate::domain::events::*;
use crate::domain::value_objects::*;
use webauthn_rs::prelude::Passkey;

#[derive(Debug, Clone)]
pub struct UserAggregate {
    user: User,
    credentials: Vec<Credential>,
    pending_events: Vec<DomainEvent<UserDomainEvent>>,
}

#[derive(Debug, Clone)]
pub enum UserDomainEvent {
    UserRegistered(UserRegistered),
    UserLoggedIn(UserLoggedIn),
    CredentialAdded(CredentialAdded),
    UserSuspended(UserSuspended),
    UserActivated(UserActivated),
}

impl UserAggregate {
    pub fn register(email: Email, display_name: DisplayName, role: UserRole, passkey: Passkey) -> Self {
        let user = User::new(email.clone(), display_name, role);
        let credential = Credential::new(user.id().clone(), passkey);
        
        let mut aggregate = Self {
            user: user.clone(),
            credentials: vec![credential.clone()],
            pending_events: Vec::new(),
        };
        
        aggregate.pending_events.push(DomainEvent::new(
            UserDomainEvent::UserRegistered(UserRegistered {
                user_id: user.id().clone(),
                email,
                role,
            })
        ));
        
        aggregate.pending_events.push(DomainEvent::new(
            UserDomainEvent::CredentialAdded(CredentialAdded {
                user_id: user.id().clone(),
                credential_id: credential.credential_id().to_vec(),
            })
        ));
        
        aggregate
    }
    
    pub fn from_persistence(user: User, credentials: Vec<Credential>) -> Self {
        Self {
            user,
            credentials,
            pending_events: Vec::new(),
        }
    }
    
    pub fn user(&self) -> &User {
        &self.user
    }
    
    pub fn credentials(&self) -> &[Credential] {
        &self.credentials
    }
    
    pub fn add_credential(&mut self, passkey: Passkey) {
        let credential = Credential::new(self.user.id().clone(), passkey);
        
        self.pending_events.push(DomainEvent::new(
            UserDomainEvent::CredentialAdded(CredentialAdded {
                user_id: self.user.id().clone(),
                credential_id: credential.credential_id().to_vec(),
            })
        ));
        
        self.credentials.push(credential);
    }
    
    pub fn record_login(&mut self) {
        self.pending_events.push(DomainEvent::new(
            UserDomainEvent::UserLoggedIn(UserLoggedIn {
                user_id: self.user.id().clone(),
                email: self.user.email().clone(),
            })
        ));
    }
    
    pub fn suspend(&mut self, reason: String) {
        self.user.suspend();
        
        self.pending_events.push(DomainEvent::new(
            UserDomainEvent::UserSuspended(UserSuspended {
                user_id: self.user.id().clone(),
                reason,
            })
        ));
    }
    
    pub fn activate(&mut self) {
        self.user.activate();
        
        self.pending_events.push(DomainEvent::new(
            UserDomainEvent::UserActivated(UserActivated {
                user_id: self.user.id().clone(),
            })
        ));
    }
    
    pub fn take_pending_events(&mut self) -> Vec<DomainEvent<UserDomainEvent>> {
        std::mem::take(&mut self.pending_events)
    }
}
