use crate::domain::value_objects::*;

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    display_name: DisplayName,
    roles: Vec<UserRole>,
    status: UserStatus,
}

impl User {
    pub fn new(email: Email, display_name: DisplayName, roles: Vec<UserRole>) -> Self {
        Self {
            id: UserId::new(),
            email,
            display_name,
            roles,
            status: UserStatus::Active,
        }
    }
    
    pub fn from_persistence(
        id: UserId,
        email: Email,
        display_name: DisplayName,
        roles: Vec<UserRole>,
        status: UserStatus,
    ) -> Self {
        Self {
            id,
            email,
            display_name,
            roles,
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
    

    pub fn roles(&self) -> &Vec<UserRole> {
        &self.roles
    }

    pub fn has_role(&self, role: UserRole) -> bool {
        self.roles.contains(&role)
    }
    
    pub fn status(&self) -> UserStatus {
        self.status
    }
    
    // Removed unused methods is_active, suspend, and activate
}
