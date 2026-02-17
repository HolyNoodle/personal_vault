use egui::FontFamily;
use libc::{MAP_SHARED, MAP_FAILED, PROT_READ, PROT_WRITE, munmap, mmap};
use epaint::{ClippedPrimitive, Primitive};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

/// Messages sent from the backend to the native app over the control socket.
/// Wire format: 4-byte little-endian length prefix + JSON body.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppMessage {
    Init { width: u32, height: u32, framerate: u32 },
    PointerMove { x: f32, y: f32 },
    PointerButton { x: f32, y: f32, button: u8, pressed: bool },
    KeyEvent { key: String, pressed: bool },
    Resize { width: u32, height: u32 },
    Shutdown,
}

/// Trait implemented by sandboxed applications.
pub trait SandboxApp: Default + Send + 'static {
    fn show(&mut self, ctx: &egui::Context);
}

// Safety wrapper so the mmap pointer can be sent to the render loop.
struct MmapMut {
    ptr: *mut u8,
    size: usize,
}
unsafe impl Send for MmapMut {}

impl MmapMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl Drop for MmapMut {
    fn drop(&mut self) {
        unsafe {
            munmap(self.ptr as *mut libc::c_void, self.size);
        }
    }
}

/// Entry point for a sandboxed native application.
///
/// Reads env vars:
/// - `SANDBOX_FB_FD`     — memfd fd for the shared RGBA framebuffer (PROT_RW, MAP_SHARED)
/// - `SANDBOX_CTRL_FD`   — Unix socket fd for receiving `AppMessage` commands
/// - `SANDBOX_WIDTH`     — framebuffer width in pixels (default 800)
/// - `SANDBOX_HEIGHT`    — framebuffer height in pixels (default 600)
/// - `SANDBOX_FRAMERATE` — target frames per second (default 30)
pub fn run<A: SandboxApp>() {
    let fb_fd: i32 = std::env::var("SANDBOX_FB_FD")
        .expect("SANDBOX_FB_FD not set")
        .parse()
        .expect("SANDBOX_FB_FD must be an integer fd");
    let ctrl_fd: i32 = std::env::var("SANDBOX_CTRL_FD")
        .expect("SANDBOX_CTRL_FD not set")
        .parse()
        .expect("SANDBOX_CTRL_FD must be an integer fd");
    let width: u32 = std::env::var("SANDBOX_WIDTH")
        .unwrap_or_else(|_| "800".to_string())
        .parse()
        .unwrap_or(800);
    let height: u32 = std::env::var("SANDBOX_HEIGHT")
        .unwrap_or_else(|_| "600".to_string())
        .parse()
        .unwrap_or(600);
    let framerate: u32 = std::env::var("SANDBOX_FRAMERATE")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .unwrap_or(30);

    let fb_size = (width * height * 4) as usize;

    // mmap the shared framebuffer (read+write so we can render into it)
    let fb_ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            fb_size,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            fb_fd,
            0,
        )
    };
    assert_ne!(
        fb_ptr,
        MAP_FAILED,
        "mmap of SANDBOX_FB_FD failed: {}",
        std::io::Error::last_os_error()
    );

    let mut framebuffer = MmapMut {
        ptr: fb_ptr as *mut u8,
        size: fb_size,
    };

    // Spawn a background thread that blocks on the control socket and
    // forwards complete AppMessages into the render-loop channel.
    let (msg_tx, msg_rx) = std::sync::mpsc::channel::<AppMessage>();
    std::thread::spawn(move || {
        let mut stream = unsafe { UnixStream::from_raw_fd(ctrl_fd) };
        loop {
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).is_err() {
                break;
            }
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut msg_buf = vec![0u8; len];
            if stream.read_exact(&mut msg_buf).is_err() {
                break;
            }
            if let Ok(msg) = serde_json::from_slice::<AppMessage>(&msg_buf) {
                if msg_tx.send(msg).is_err() {
                    break;
                }
            }
        }
    });

    // Initialise egui context
    let ctx = egui::Context::default();
    setup_custom_fonts(&ctx);

    let mut app = A::default();
    let mut font_texture: Option<egui::ColorImage> = None;
    let mut pointer_pos = egui::Pos2::ZERO;
    let mut pointer_pressed = false;

    let frame_interval = Duration::from_millis((1000u64 / framerate.max(1) as u64).max(16));

    loop {
        let start = Instant::now();
        let mut shutdown = false;
        let mut events: Vec<egui::Event> = Vec::new();

        // Drain all messages that arrived since the last frame
        loop {
            match msg_rx.try_recv() {
                Ok(msg) => match msg {
                    AppMessage::Shutdown => {
                        shutdown = true;
                        break;
                    }
                    AppMessage::PointerMove { x, y } => {
                        pointer_pos = egui::Pos2::new(x, y);
                        events.push(egui::Event::PointerMoved(pointer_pos));
                    }
                    AppMessage::PointerButton { x, y, button, pressed } => {
                        pointer_pos = egui::Pos2::new(x, y);
                        pointer_pressed = pressed && button == 0;
                        events.push(egui::Event::PointerButton {
                            pos: pointer_pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                    AppMessage::KeyEvent { key, pressed } => {
                        if let Some(egui_key) = map_key_to_egui(&key) {
                            events.push(egui::Event::Key {
                                key: egui_key,
                                physical_key: None,
                                pressed,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            });
                        }
                        if pressed && key.len() == 1 {
                            events.push(egui::Event::Text(key));
                        }
                    }
                    AppMessage::Resize { .. } | AppMessage::Init { .. } => {}
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    shutdown = true;
                    break;
                }
            }
        }

        if shutdown {
            break;
        }

        // Build raw egui input
        let mut raw_input = egui::RawInput::default();
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
        raw_input.events.extend(events);

        // Run egui
        let full_output = ctx.run(raw_input, |ctx| app.show(ctx));

        // Update cached font texture
        for (tex_id, delta) in &full_output.textures_delta.set {
            if *tex_id == egui::TextureId::default() {
                match &delta.image {
                    egui::ImageData::Font(font_image) => {
                        let size = delta.image.size();
                        let pixels: Vec<egui::Color32> =
                            font_image.srgba_pixels(None).collect();
                        font_texture = Some(egui::ColorImage {
                            size: [size[0], size[1]],
                            pixels,
                        });
                    }
                    egui::ImageData::Color(color_image) => {
                        font_texture = Some(color_image.as_ref().clone());
                    }
                }
            }
        }

        // Tessellate shapes into triangle meshes
        let clipped_primitives = ctx.tessellate(full_output.shapes, 1.0);

        // Software-rasterize into the shared framebuffer
        render_to_buf(
            &clipped_primitives,
            &font_texture,
            framebuffer.as_mut_slice(),
            width as usize,
            height as usize,
            pointer_pos,
        );

        // Maintain target framerate
        let elapsed = start.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
    }
}

/// Set up embedded fonts on the given egui context.
pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    const DEJAVU_FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    const LIBERATION_FONT: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");

    fonts.font_data.insert(
        "DejaVuSans".to_owned(),
        egui::FontData::from_static(DEJAVU_FONT),
    );
    fonts.font_data.insert(
        "LiberationSans".to_owned(),
        egui::FontData::from_static(LIBERATION_FONT),
    );

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.clear();
        family.push("DejaVuSans".to_owned());
        family.push("LiberationSans".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.clear();
        family.push("DejaVuSans".to_owned());
        family.push("LiberationSans".to_owned());
    }

    ctx.set_fonts(fonts);
}

/// Software rasterize egui primitives into a raw RGBA framebuffer slice.
pub fn render_to_buf(
    clipped_primitives: &[ClippedPrimitive],
    font_texture: &Option<egui::ColorImage>,
    fb: &mut [u8],
    width: usize,
    height: usize,
    pointer_pos: egui::Pos2,
) {
    // Clear to opaque black
    for chunk in fb.chunks_exact_mut(4) {
        chunk[0] = 30;
        chunk[1] = 30;
        chunk[2] = 30;
        chunk[3] = 255;
    }

    for cp in clipped_primitives {
        let clip = cp.clip_rect;
        if let Primitive::Mesh(mesh) = &cp.primitive {
            for tri in mesh.indices.chunks(3) {
                if tri.len() < 3 {
                    continue;
                }
                let v0 = &mesh.vertices[tri[0] as usize];
                let v1 = &mesh.vertices[tri[1] as usize];
                let v2 = &mesh.vertices[tri[2] as usize];

                let min_x = v0.pos.x.min(v1.pos.x).min(v2.pos.x)
                    .max(clip.min.x).max(0.0) as i32;
                let max_x = v0.pos.x.max(v1.pos.x).max(v2.pos.x)
                    .min(clip.max.x).min(width as f32 - 1.0) as i32;
                let min_y = v0.pos.y.min(v1.pos.y).min(v2.pos.y)
                    .max(clip.min.y).max(0.0) as i32;
                let max_y = v0.pos.y.max(v1.pos.y).max(v2.pos.y)
                    .min(clip.max.y).min(height as f32 - 1.0) as i32;

                for py in min_y..=max_y {
                    for px in min_x..=max_x {
                        let p = egui::Pos2::new(px as f32 + 0.5, py as f32 + 0.5);
                        let (w0, w1, w2) = barycentric(p, v0.pos, v1.pos, v2.pos);
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }

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

                        if let Some(ref tex) = font_texture {
                            let u = w0 * v0.uv.x + w1 * v1.uv.x + w2 * v2.uv.x;
                            let v = w0 * v0.uv.y + w1 * v1.uv.y + w2 * v2.uv.y;
                            let tx = ((u * tex.size[0] as f32) as usize)
                                .min(tex.size[0].saturating_sub(1));
                            let ty = ((v * tex.size[1] as f32) as usize)
                                .min(tex.size[1].saturating_sub(1));
                            let tex_pixel = tex.pixels[ty * tex.size[0] + tx];
                            a = a * tex_pixel.a() as f32 / 255.0;
                        }

                        let a_norm = (a / 255.0).clamp(0.0, 1.0);
                        if a_norm < 0.004 {
                            continue;
                        }

                        let idx = (py as usize * width + px as usize) * 4;
                        if idx + 3 >= fb.len() {
                            continue;
                        }

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
    }

    // Draw a crosshair cursor
    let cx = pointer_pos.x as i32;
    let cy = pointer_pos.y as i32;
    let cursor_size: i32 = 10;
    for dy in -cursor_size..=cursor_size {
        for dx in -cursor_size..=cursor_size {
            if dx == 0 || dy == 0 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let idx = (py as usize * width + px as usize) * 4;
                    if idx + 3 < fb.len() {
                        if dx.abs() <= 1 && dy.abs() <= 1 {
                            fb[idx] = 0;
                            fb[idx + 1] = 0;
                            fb[idx + 2] = 0;
                            fb[idx + 3] = 255;
                        } else {
                            fb[idx] = 255;
                            fb[idx + 1] = 255;
                            fb[idx + 2] = 255;
                            fb[idx + 3] = 255;
                        }
                    }
                }
            }
        }
    }
}

fn barycentric(
    p: egui::Pos2,
    a: egui::Pos2,
    b: egui::Pos2,
    c: egui::Pos2,
) -> (f32, f32, f32) {
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
        return (-1.0, -1.0, -1.0);
    }
    let inv = 1.0 / denom;
    let w1 = (d11 * d20 - d01 * d21) * inv;
    let w2 = (d00 * d21 - d01 * d20) * inv;
    (1.0 - w1 - w2, w1, w2)
}

fn map_key_to_egui(key: &str) -> Option<egui::Key> {
    match key {
        "Enter" => Some(egui::Key::Enter),
        "Escape" => Some(egui::Key::Escape),
        "Backspace" => Some(egui::Key::Backspace),
        "Delete" => Some(egui::Key::Delete),
        "Tab" => Some(egui::Key::Tab),
        "ArrowLeft" => Some(egui::Key::ArrowLeft),
        "ArrowRight" => Some(egui::Key::ArrowRight),
        "ArrowUp" => Some(egui::Key::ArrowUp),
        "ArrowDown" => Some(egui::Key::ArrowDown),
        "Home" => Some(egui::Key::Home),
        "End" => Some(egui::Key::End),
        "PageUp" => Some(egui::Key::PageUp),
        "PageDown" => Some(egui::Key::PageDown),
        "Space" | " " => Some(egui::Key::Space),
        s if s.len() == 1 => {
            let ch = s.chars().next()?;
            match ch {
                'a' | 'A' => Some(egui::Key::A),
                'b' | 'B' => Some(egui::Key::B),
                'c' | 'C' => Some(egui::Key::C),
                'd' | 'D' => Some(egui::Key::D),
                'e' | 'E' => Some(egui::Key::E),
                'f' | 'F' => Some(egui::Key::F),
                'g' | 'G' => Some(egui::Key::G),
                'h' | 'H' => Some(egui::Key::H),
                'i' | 'I' => Some(egui::Key::I),
                'j' | 'J' => Some(egui::Key::J),
                'k' | 'K' => Some(egui::Key::K),
                'l' | 'L' => Some(egui::Key::L),
                'm' | 'M' => Some(egui::Key::M),
                'n' | 'N' => Some(egui::Key::N),
                'o' | 'O' => Some(egui::Key::O),
                'p' | 'P' => Some(egui::Key::P),
                'q' | 'Q' => Some(egui::Key::Q),
                'r' | 'R' => Some(egui::Key::R),
                's' | 'S' => Some(egui::Key::S),
                't' | 'T' => Some(egui::Key::T),
                'u' | 'U' => Some(egui::Key::U),
                'v' | 'V' => Some(egui::Key::V),
                'w' | 'W' => Some(egui::Key::W),
                'x' | 'X' => Some(egui::Key::X),
                'y' | 'Y' => Some(egui::Key::Y),
                'z' | 'Z' => Some(egui::Key::Z),
                '0' => Some(egui::Key::Num0),
                '1' => Some(egui::Key::Num1),
                '2' => Some(egui::Key::Num2),
                '3' => Some(egui::Key::Num3),
                '4' => Some(egui::Key::Num4),
                '5' => Some(egui::Key::Num5),
                '6' => Some(egui::Key::Num6),
                '7' => Some(egui::Key::Num7),
                '8' => Some(egui::Key::Num8),
                '9' => Some(egui::Key::Num9),
                _ => None,
            }
        }
        _ => None,
    }
}
