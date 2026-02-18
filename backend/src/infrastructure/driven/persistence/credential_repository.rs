use std::sync::Arc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use async_trait::async_trait;
use crate::application::ports::CredentialRepository;
use crate::domain::{Credential, UserId};
use crate::infrastructure::driven::persistence::schema::webauthn_credentials;
use crate::infrastructure::driven::persistence::db_types::{DbCredential, NewDbCredential};

pub struct SqliteCredentialRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl SqliteCredentialRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self { pool }
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[async_trait]
impl CredentialRepository for SqliteCredentialRepository {
    async fn find_by_user_id(&self, user_id: &UserId) -> Result<Vec<Credential>, String> {
        let user_id_str = user_id.to_string();
        let user_id_clone = user_id.clone();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let rows: Vec<DbCredential> = webauthn_credentials::table
                .filter(webauthn_credentials::user_id.eq(&user_id_str))
                .load::<DbCredential>(&mut conn)
                .map_err(|e| format!("Database error: {}", e))?;

            let credentials = rows
                .into_iter()
                .filter_map(|row| {
                    let cred_id_bytes = hex_to_bytes(&row.credential_id).ok()?;
                    let passkey = serde_json::from_str(&row.public_key).ok()?;
                    // Convert i64 from DB to u32 for domain
                    let sign_count = u32::try_from(row.sign_count).ok()?;
                    Some(Credential::from_persistence(
                        user_id_clone.clone(),
                        cred_id_bytes,
                        passkey,
                        sign_count,
                    ))
                })
                .collect();

            Ok(credentials)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn save(&self, credential: &Credential) -> Result<(), String> {
        let id_val = uuid::Uuid::new_v4().to_string();
        let user_id_val = credential.user_id().to_string();
        let credential_id_val = bytes_to_hex(credential.credential_id());
        let public_key_val = serde_json::to_string(credential.passkey())
            .map_err(|e| format!("Failed to serialize passkey: {}", e))?;
        let sign_count_val = credential.sign_count() as i64;
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let new_cred = NewDbCredential {
                id: id_val,
                user_id: user_id_val,
                credential_id: credential_id_val,
                public_key: public_key_val,
                sign_count: sign_count_val,
            };
            use diesel::dsl::insert_into;
            use diesel::sqlite::Sqlite;
            use diesel::query_builder::InsertStatement;
            use diesel::query_dsl::RunQueryDsl;
            use crate::infrastructure::driven::persistence::schema::webauthn_credentials::dsl::*;
            diesel::sql_query("INSERT INTO webauthn_credentials (id, user_id, credential_id, public_key, sign_count) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(credential_id) DO UPDATE SET sign_count=excluded.sign_count, public_key=excluded.public_key")
                .bind::<diesel::sql_types::Text, _>(&new_cred.id)
                .bind::<diesel::sql_types::Text, _>(&new_cred.user_id)
                .bind::<diesel::sql_types::Text, _>(&new_cred.credential_id)
                .bind::<diesel::sql_types::Text, _>(&new_cred.public_key)
                .bind::<diesel::sql_types::BigInt, _>(new_cred.sign_count)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to upsert credential: {}", e))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
