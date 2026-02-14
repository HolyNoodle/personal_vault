pub mod auth;
pub mod files;
pub mod video_api;

pub use auth::setup_routes as auth_routes;
pub use files::files_routes;
