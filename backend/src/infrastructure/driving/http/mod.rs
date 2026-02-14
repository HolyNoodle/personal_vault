pub mod auth;
pub mod files;

pub use auth::setup_routes as auth_routes;
pub use files::files_routes;
