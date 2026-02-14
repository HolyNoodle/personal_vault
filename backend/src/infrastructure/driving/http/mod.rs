pub mod auth;
pub mod files;
pub mod video_api;
pub mod application_routes;

pub use auth::setup_routes as auth_routes;
pub use files::files_routes;
pub use application_routes::*;
