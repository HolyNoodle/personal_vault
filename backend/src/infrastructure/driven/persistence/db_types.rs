use diesel::prelude::*;
use crate::infrastructure::driven::persistence::schema::{users, webauthn_credentials};

#[derive(diesel::QueryableByName, Debug)]
pub struct DbInvitation {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub owner_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub invitee_email: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub token: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub granted_paths: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub status: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub expires_at: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub created_at: String,
}

#[derive(diesel::QueryableByName, Debug)]
pub struct DbFilePermission {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub owner_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub client_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub path: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub access: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub granted_at: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub expires_at: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub revoked_at: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
pub struct DbUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub roles: String,
    pub status: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewDbUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub roles: String,
    pub status: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = webauthn_credentials)]
pub struct DbCredential {
    pub id: String,
    pub user_id: String,
    pub credential_id: String,
    pub public_key: String,
    pub sign_count: i64,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = webauthn_credentials)]
pub struct NewDbCredential {
    pub id: String,
    pub user_id: String,
    pub credential_id: String,
    pub public_key: String,
    pub sign_count: i64,
}
