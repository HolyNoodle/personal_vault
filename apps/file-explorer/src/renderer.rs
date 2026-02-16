use egui::FontFamily;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Embed DejaVuSans.ttf and LiberationSans-Regular.ttf directly in WASM binary
    const DEJAVU_FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    const LIBERATION_FONT: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");
    log_wasm(&format!("[wasm] setup_custom_fonts: DejaVuSans.ttf bytes = {}", DEJAVU_FONT.len()));
    log_wasm(&format!("[wasm] setup_custom_fonts: LiberationSans-Regular.ttf bytes = {}", LIBERATION_FONT.len()));
    if DEJAVU_FONT.len() == 0 {
        log_wasm("[wasm] WARNING: DejaVuSans.ttf is empty!");
    }
    if LIBERATION_FONT.len() == 0 {
        log_wasm("[wasm] WARNING: LiberationSans-Regular.ttf is empty!");
    }

    fonts.font_data.insert(
        "DejaVuSans".to_owned(),
        egui::FontData::from_static(DEJAVU_FONT),
    );
    fonts.font_data.insert(
        "LiberationSans".to_owned(),
        egui::FontData::from_static(LIBERATION_FONT),
    );

    // Set the font for Proportional and Monospace, with fallback
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.clear();
        family.push("DejaVuSans".to_owned());
        family.push("LiberationSans".to_owned());
    } else {
        log_wasm("[wasm] WARNING: Proportional font family not found!");
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.clear();
        family.push("DejaVuSans".to_owned());
        family.push("LiberationSans".to_owned());
    } else {
        log_wasm("[wasm] WARNING: Monospace font family not found!");
    }

    ctx.set_fonts(fonts);
    log_wasm("[wasm] Fonts successfully initialized");
}

extern "C" {
    fn console_log(ptr: *const u8, len: usize);
}

fn log_wasm(msg: &str) {
    unsafe {
        console_log(msg.as_ptr(), msg.len());
    }
}
#[no_mangle]
pub extern "C" fn set_size(w: i32, h: i32) {
    log_wasm(&format!("[wasm] set_size ENTRY: width={}, height={}", w, h));
    let mut width = WIDTH.lock().unwrap();
    let mut height = HEIGHT.lock().unwrap();
    let mut fb = FRAMEBUFFER.lock().unwrap();
    if w > 0 && h > 0 && (*width != w as usize || *height != h as usize) {
        log_wasm(&format!("[wasm] set_size resizing: old=({},{}), new=({},{}), fb_len={}", *width, *height, w, h, fb.len()));
        *width = w as usize;
        *height = h as usize;
        fb.resize(*width * *height * 4, 0);
        log_wasm(&format!("[wasm] set_size resized: width={}, height={}, fb_len={}", *width, *height, fb.len()));
    } else {
        log_wasm(&format!("[wasm] set_size no resize needed: width={}, height={}, fb_len={}", *width, *height, fb.len()));
    }
    log_wasm("[wasm] set_size EXIT");
}
use crate::app::{create_file_explorer_app, FileExplorerApp};
use egui;
use epaint::{ClippedPrimitive, Primitive};
use once_cell::sync::Lazy;
use std::sync::Mutex;

static WIDTH: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(800));
static HEIGHT: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(600));
static FRAMERATE: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(30));

fn framebuffer_size() -> usize {
    *WIDTH.lock().unwrap() * *HEIGHT.lock().unwrap() * 4
}

static FRAMEBUFFER: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(vec![0u8; 800 * 600 * 4]));
static APP: Lazy<Mutex<FileExplorerApp>> = Lazy::new(|| Mutex::new(create_file_explorer_app()));
static CTX: Lazy<egui::Context> = Lazy::new(|| {
    log_wasm("[wasm] ===== CTX LAZY INITIALIZATION STARTING =====");
    let ctx = egui::Context::default();
    log_wasm("[wasm] Context::default() created");
    setup_custom_fonts(&ctx);
    log_wasm("[wasm] Context created with fonts initialized");
    ctx
});
static FONT_TEXTURE: Lazy<Mutex<Option<egui::ColorImage>>> = Lazy::new(|| Mutex::new(None));

// Input state forwarded from host
static POINTER_POS: Lazy<Mutex<egui::Pos2>> =
    Lazy::new(|| Mutex::new(egui::Pos2::new(0.0, 0.0)));
static POINTER_PRESSED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

#[no_mangle]
pub extern "C" fn render_file_explorer_frame() {
    use std::panic;
    log_wasm("[wasm] render_file_explorer_frame: BEGIN WRAPPED");
    let result = panic::catch_unwind(|| {
        let mut app = APP.lock().unwrap();
        log_wasm("[wasm] ===== ABOUT TO ACCESS CTX =====");
        let ctx = &*CTX;
        log_wasm("[wasm] ===== CTX ACCESSED SUCCESSFULLY =====");
        log_wasm("[wasm] render_file_explorer_frame ENTRY");
        let pointer_pos = *POINTER_POS.lock().unwrap();
        let pointer_pressed = *POINTER_PRESSED.lock().unwrap();

        // Build raw input with current pointer state
        let mut raw_input = egui::RawInput::default();
        let width = *WIDTH.lock().unwrap();
        let height = *HEIGHT.lock().unwrap();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width as f32, height as f32),
        ));
        raw_input.events.push(egui::Event::PointerMoved(pointer_pos));
        if pointer_pressed {
            raw_input.events.push(egui::Event::PointerButton {
                pos: pointer_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
        }

        // Run egui: this executes the UI logic and returns shapes to paint
        let full_output = ctx.run(raw_input, |ctx| {
            log_wasm("[wasm] render_file_explorer_frame: calling app.show");
            app.show(ctx);
            log_wasm("[wasm] render_file_explorer_frame: returned from app.show");
        });

        // Tessellate shapes into triangle meshes
        let pixels_per_point = 1.0;
        let clipped_primitives: Vec<ClippedPrimitive> =
            ctx.tessellate(full_output.shapes, pixels_per_point);

        // Software rasterize into RGBA buffer
        let mut fb = FRAMEBUFFER.lock().unwrap();

        // Clear to red for debugging
        for chunk in fb.chunks_exact_mut(4) {
            chunk[0] = 255; // R
            chunk[1] = 0;   // G
            chunk[2] = 0;   // B
            chunk[3] = 255; // A
        }

        let textures = &full_output.textures_delta;
        // We only handle the font texture (id = Managed(0)) for text rendering
        log_wasm(&format!("[wasm] render_file_explorer_frame: width={}, height={}, fb_len={}", width, height, fb.len()));

        // Update font texture if there's a new one
        log_wasm("[wasm] render_file_explorer_frame: before font texture loop");
        for (tex_id, delta) in &textures.set {
            if *tex_id == egui::TextureId::default() {
                if let egui::ImageData::Font(font_image) = &delta.image {
                    let size = delta.image.size();
                    log_wasm(&format!("[wasm] Font texture generated: size={:?}", size));
                    let pixels: Vec<egui::Color32> = font_image.srgba_pixels(None).collect();

                    // Debug: Check first few pixels of font texture
                    if pixels.len() > 10 {
                        log_wasm(&format!("[wasm] Font texture first 10 pixels: {:?}", &pixels[0..10]));
                    }

                    *FONT_TEXTURE.lock().unwrap() = Some(egui::ColorImage {
                        size: [size[0], size[1]],
                        pixels,
                    });
                    log_wasm("[wasm] render_file_explorer_frame: after font texture generation");
                } else if let egui::ImageData::Color(color_image) = &delta.image {
                    log_wasm("[wasm] Color image texture found in textures.set (unexpected for font)");
                    *FONT_TEXTURE.lock().unwrap() = Some(color_image.as_ref().clone());
                }
            }
        }

        // Clone the font texture for rendering (don't hold lock during render)
        let font_texture = FONT_TEXTURE.lock().unwrap().clone();

        log_wasm("[wasm] render_file_explorer_frame: before primitive loop");
        for cp in &clipped_primitives {
            let clip = cp.clip_rect;
            let width = *WIDTH.lock().unwrap();
            let height = *HEIGHT.lock().unwrap();
            match &cp.primitive {
                Primitive::Mesh(mesh) => {
                    // Rasterize each triangle in the mesh
                    for tri in mesh.indices.chunks(3) {
                        if tri.len() < 3 {
                            continue;
                        }
                        let v0 = &mesh.vertices[tri[0] as usize];
                        let v1 = &mesh.vertices[tri[1] as usize];
                        let v2 = &mesh.vertices[tri[2] as usize];

                        // Compute bounding box
                        let min_x = v0.pos.x.min(v1.pos.x).min(v2.pos.x).max(clip.min.x).max(0.0) as i32;
                        let max_x = v0.pos.x.max(v1.pos.x).max(v2.pos.x).min(clip.max.x).min(width as f32 - 1.0) as i32;
                        let min_y = v0.pos.y.min(v1.pos.y).min(v2.pos.y).max(clip.min.y).max(0.0) as i32;
                        let max_y = v0.pos.y.max(v1.pos.y).max(v2.pos.y).min(clip.max.y).min(height as f32 - 1.0) as i32;

                        for py in min_y..=max_y {
                            for px in min_x..=max_x {
                                let p = egui::Pos2::new(px as f32 + 0.5, py as f32 + 0.5);

                                // Barycentric coordinates
                                let (w0, w1, w2) =
                                    barycentric(p, v0.pos, v1.pos, v2.pos);
                                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                                    continue;
                                }

                                // Interpolate color
                                let r = w0 * v0.color.r() as f32
                                    + w1 * v1.color.r() as f32
                                    + w2 * v2.color.r() as f32;
                                let g = w0 * v0.color.g() as f32
                                    + w1 * v1.color.g() as f32
                                    + w2 * v2.color.g() as f32;
                                let b = w0 * v0.color.b() as f32
                                    + w1 * v1.color.b() as f32
                                    + w2 * v2.color.b() as f32;
                                let mut a = w0 * v0.color.a() as f32
                                    + w1 * v1.color.a() as f32
                                    + w2 * v2.color.a() as f32;

                                // Sample font texture if available (for text rendering)
                                if let Some(ref tex) = font_texture {
                                    let u = w0 * v0.uv.x + w1 * v1.uv.x + w2 * v2.uv.x;
                                    let v = w0 * v0.uv.y + w1 * v1.uv.y + w2 * v2.uv.y;
                                    let tx = ((u * tex.size[0] as f32) as usize).min(tex.size[0].saturating_sub(1));
                                    let ty = ((v * tex.size[1] as f32) as usize).min(tex.size[1].saturating_sub(1));
                                    let tex_pixel = tex.pixels[ty * tex.size[0] + tx];
                                    // Multiply vertex alpha with texture alpha
                                    a = a * tex_pixel.a() as f32 / 255.0;
                                }

                                let a_norm = (a / 255.0).clamp(0.0, 1.0);
                                if a_norm < 0.004 {
                                    continue; // Skip nearly transparent
                                }

                                let idx = (py as usize * width + px as usize) * 4;
                                if idx + 3 >= fb.len() {
                                    continue;
                                }

                                // Alpha blend over existing pixel
                                let dst_r = fb[idx] as f32;
                                let dst_g = fb[idx + 1] as f32;
                                let dst_b = fb[idx + 2] as f32;

                                fb[idx] = (r * a_norm + dst_r * (1.0 - a_norm)).clamp(0.0, 255.0) as u8;
                                fb[idx + 1] = (g * a_norm + dst_g * (1.0 - a_norm)).clamp(0.0, 255.0) as u8;
                                fb[idx + 2] = (b * a_norm + dst_b * (1.0 - a_norm)).clamp(0.0, 255.0) as u8;
                                fb[idx + 3] = 255;
                            }
                        }
                    }
                }
                Primitive::Callback(_) => {
                    // Paint callbacks not supported in software renderer
                }
            }
        }
        log_wasm("[wasm] render_file_explorer_frame: END OF WRAPPED");
    });
    if let Err(_e) = result {
        log_wasm("[wasm] PANIC CAUGHT in render_file_explorer_frame");
    }
}

/// Compute barycentric coordinates for point p in triangle (a, b, c)
fn barycentric(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> (f32, f32, f32) {
    let v0 = egui::Vec2::new(b.x - a.x, b.y - a.y);
    let v1 = egui::Vec2::new(c.x - a.x, c.y - a.y);
    let v2 = egui::Vec2::new(p.x - a.x, p.y - a.y);

    let d00 = v0.x * v0.x + v0.y * v0.y;
    let d01 = v0.x * v1.x + v0.y * v1.y;
    let d11 = v1.x * v1.x + v1.y * v1.y;
    let d20 = v2.x * v0.x + v2.y * v0.y;
    let d21 = v2.x * v1.x + v2.y * v1.y;

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-10 {
        return (-1.0, -1.0, -1.0); // Degenerate triangle
    }

    let inv_denom = 1.0 / denom;
    let w1 = (d11 * d20 - d01 * d21) * inv_denom;
    let w2 = (d00 * d21 - d01 * d20) * inv_denom;
    let w0 = 1.0 - w1 - w2;

    (w0, w1, w2)
}


#[no_mangle]
pub extern "C" fn get_framebuffer_ptr() -> *const u8 {
    FRAMEBUFFER.lock().unwrap().as_ptr()
}

#[no_mangle]
pub extern "C" fn get_framebuffer_size() -> usize {
    framebuffer_size()
}

#[no_mangle]
pub extern "C" fn get_width() -> u32 {
    *WIDTH.lock().unwrap() as u32
}

#[no_mangle]
pub extern "C" fn get_height() -> u32 {
    *HEIGHT.lock().unwrap() as u32
}

#[no_mangle]
pub extern "C" fn get_framerate() -> u32 {
    *FRAMERATE.lock().unwrap()
}

#[no_mangle]
pub extern "C" fn set_width(w: i32) {
    log_wasm(&format!("[wasm] set_width ENTRY: width={}", w));
    let mut width = WIDTH.lock().unwrap();
    let mut fb = FRAMEBUFFER.lock().unwrap();
    if *width != w as usize && w > 0 {
        log_wasm(&format!("[wasm] set_width resizing: old={}, new={}, fb_len={}", *width, w, fb.len()));
        *width = w as usize;
        fb.resize(*width * *HEIGHT.lock().unwrap() * 4, 0);
        log_wasm(&format!("[wasm] set_width resized: width={}, fb_len={}", *width, fb.len()));
    } else {
        log_wasm(&format!("[wasm] set_width no resize needed: width={}, fb_len={}", *width, fb.len()));
    }
    log_wasm("[wasm] set_width EXIT");
}

#[no_mangle]
pub extern "C" fn set_height(h: i32) {
    log_wasm(&format!("[wasm] set_height ENTRY: height={}", h));
    let mut height = HEIGHT.lock().unwrap();
    let mut fb = FRAMEBUFFER.lock().unwrap();
    if *height != h as usize && h > 0 {
        log_wasm(&format!("[wasm] set_height resizing: old={}, new={}, fb_len={}", *height, h, fb.len()));
        *height = h as usize;
        fb.resize(*WIDTH.lock().unwrap() * *height * 4, 0);
        log_wasm(&format!("[wasm] set_height resized: height={}, fb_len={}", *height, fb.len()));
    } else {
        log_wasm(&format!("[wasm] set_height no resize needed: height={}, fb_len={}", *height, fb.len()));
    }
    log_wasm("[wasm] set_height EXIT");
}

#[no_mangle]
pub extern "C" fn set_framerate(fps: i32) {
    log_wasm(&format!("[wasm] set_framerate ENTRY: fps={}", fps));
    let mut fr = FRAMERATE.lock().unwrap();
    if fps > 0 {
        *fr = fps as u32;
        log_wasm(&format!("[wasm] set_framerate set: framerate={}", *fr));
    } else {
        log_wasm(&format!("[wasm] set_framerate ignored: fps={} (<=0)", fps));
    }
    log_wasm("[wasm] set_framerate EXIT");
}

#[no_mangle]
pub extern "C" fn handle_pointer_event(x: f32, y: f32, pressed: u32) {
    *POINTER_POS.lock().unwrap() = egui::Pos2::new(x, y);
    *POINTER_PRESSED.lock().unwrap() = pressed != 0;
}
