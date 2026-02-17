mod app;
mod renderer;

pub use app::{FileExplorerApp, FileItem};
pub use renderer::{set_width, set_height, set_size, set_framerate, get_width, get_height, get_framerate, get_framebuffer_ptr, get_framebuffer_size};
