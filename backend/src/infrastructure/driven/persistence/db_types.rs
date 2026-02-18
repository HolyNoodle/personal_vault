use diesel::prelude::*;
use crate::infrastructure::driven::persistence::schema::{users, webauthn_credentials};

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
