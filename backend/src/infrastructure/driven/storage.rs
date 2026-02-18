use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn create_owner_storage(user_id: &str) -> std::io::Result<PathBuf> {
    let storage_root = env::var("STORAGE_PATH").unwrap();
    let user_dir = Path::new(&storage_root).join(user_id);
    fs::create_dir_all(&user_dir)?;
    Ok(user_dir)
}
