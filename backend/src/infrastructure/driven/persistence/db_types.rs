use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "user_role")]
pub enum DbUserRole {
    #[sqlx(rename = "super_admin")]
    SuperAdmin,
    #[sqlx(rename = "owner")]
    Owner,
    #[sqlx(rename = "client")]
    Client,
}

impl DbUserRole {
    pub fn to_domain(&self) -> crate::domain::UserRole {
        match self {
            DbUserRole::SuperAdmin => crate::domain::UserRole::SuperAdmin,
            DbUserRole::Owner => crate::domain::UserRole::Owner,
            DbUserRole::Client => crate::domain::UserRole::Client,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "user_status")]
pub enum DbUserStatus {
    #[sqlx(rename = "active")]
    Active,
    #[sqlx(rename = "suspended")]
    Suspended,
    #[sqlx(rename = "deleted")]
    Deleted,
}

impl DbUserStatus {
    pub fn to_domain(&self) -> crate::domain::UserStatus {
        match self {
            DbUserStatus::Active => crate::domain::UserStatus::Active,
            DbUserStatus::Suspended => crate::domain::UserStatus::Suspended,
            DbUserStatus::Deleted => crate::domain::UserStatus::Deleted,
        }
    }
}
