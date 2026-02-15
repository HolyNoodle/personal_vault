use crate::domain::value_objects::*;

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
    
    // Removed unused methods is_active, suspend, and activate
}
