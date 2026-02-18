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
                    Some(Credential::from_persistence(
                        user_id_clone.clone(),
                        cred_id_bytes,
                        passkey,
                        row.sign_count as u32,
                    ))
                })
                .collect();

            Ok(credentials)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn save(&self, credential: &Credential) -> Result<(), String> {
        let id = uuid::Uuid::new_v4().to_string();
        let user_id = credential.user_id().to_string();
        let credential_id = bytes_to_hex(credential.credential_id());
        let public_key = serde_json::to_string(credential.passkey())
            .map_err(|e| format!("Failed to serialize passkey: {}", e))?;
        let sign_count = credential.sign_count() as i64;
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            let new_cred = NewDbCredential { id, user_id, credential_id, public_key, sign_count };
            diesel::insert_into(webauthn_credentials::table)
                .values(&new_cred)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to create credential: {}", e))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
