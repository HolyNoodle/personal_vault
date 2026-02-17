use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn, debug};
use wasmtime::*;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 600;
const PIXEL_SIZE: u32 = 4; // RGBA8

/// Input event types
#[derive(Debug, Clone)]
pub enum InputEvent {
    PointerMove { x: f32, y: f32, pressed: bool },
    MouseDown { button: u8 },
    MouseUp { button: u8 },
    KeyDown { key: String, code: String },
    KeyUp { key: String, code: String },
}

/// Manages WASM application instances, replacing XvfbManager
pub struct WasmAppManager {
    sessions: Arc<RwLock<HashMap<String, WasmSession>>>,
    wasm_dir: String,
}

struct WasmSession {
    cancel_token: CancellationToken,
    frame_sender: Option<mpsc::Sender<Vec<u8>>>,
    input_sender: Option<mpsc::UnboundedSender<InputEvent>>,
    width: u32,
    height: u32,
    last_x: Arc<std::sync::Mutex<f32>>,
    last_y: Arc<std::sync::Mutex<f32>>,
    button_pressed: Arc<std::sync::Mutex<bool>>,
}

impl WasmAppManager {
    pub fn new(wasm_dir: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            wasm_dir,
        }
    }

    /// Launch a WASM app for a session and return a channel receiver for RGBA frames
    pub async fn launch_app(
        &self,
        session_id: &str,
        app_name: &str,
        width: u16,
        height: u16,
        framerate: u8,
    ) -> Result<mpsc::Receiver<Vec<u8>>> {
        let wasm_path = format!("{}/{}.wasm", self.wasm_dir, app_name.replace('-', "_"));
        info!(
            "Loading WASM app '{}' from {} for session {} ({}x{}@{}fps)",
            app_name, wasm_path, session_id, width, height, framerate
        );

        // Verify the WASM file exists
        if !std::path::Path::new(&wasm_path).exists() {
            return Err(anyhow::anyhow!("WASM file not found: {}", wasm_path));
        }

        let cancel_token = CancellationToken::new();
        let (frame_tx, frame_rx) = mpsc::channel::<Vec<u8>>(2);
        let (input_tx, input_rx) = mpsc::unbounded_channel::<InputEvent>();

        let session = WasmSession {
            cancel_token: cancel_token.clone(),
            frame_sender: Some(frame_tx.clone()),
            input_sender: Some(input_tx.clone()),
            width: width as u32,
            height: height as u32,
            last_x: Arc::new(std::sync::Mutex::new(0.0)),
            last_y: Arc::new(std::sync::Mutex::new(0.0)),
            button_pressed: Arc::new(std::sync::Mutex::new(false)),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session);
        drop(sessions);

        let session_id_owned = session_id.to_string();
        let token = cancel_token.clone();
        let frame_interval =
            std::time::Duration::from_millis((1000 / framerate.max(1) as u64).max(16));

        // Spawn the render loop in a blocking thread (wasmtime is synchronous)
        tokio::spawn(async move {
            let width = width as u32;
            let height = height as u32;
            let framerate = framerate as u8;
            let result = tokio::task::spawn_blocking(move || {
                run_wasm_render_loop(&wasm_path, frame_tx, input_rx, token, frame_interval, &session_id_owned, width, height, framerate)
            })
            .await;

            match result {
                Ok(Ok(())) => info!("WASM render loop ended cleanly"),
                Ok(Err(e)) => error!("WASM render loop error: {}", e),
                Err(e) => error!("WASM render task panicked: {}", e),
            }
        });

        Ok(frame_rx)
    }

    /// Forward a pointer event to the WASM app
    pub async fn handle_pointer_event(
        &self,
        session_id: &str,
        x: f32,
        y: f32,
        _pressed: bool,
    ) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            // Update last known position
            *session.last_x.lock().unwrap() = x;
            *session.last_y.lock().unwrap() = y;

            // Get current button state
            let pressed = *session.button_pressed.lock().unwrap();

            if let Some(input_sender) = &session.input_sender {
                let _ = input_sender.send(InputEvent::PointerMove { x, y, pressed });
            }
        }
    }

    /// Forward a mouse button event to the WASM app
    pub async fn handle_mouse_button(&self, session_id: &str, button: u8, pressed: bool) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            // Update button state
            *session.button_pressed.lock().unwrap() = pressed;

            // Get current position
            let x = *session.last_x.lock().unwrap();
            let y = *session.last_y.lock().unwrap();

            if let Some(input_sender) = &session.input_sender {
                // Send both the button event and a position update with new pressed state
                let event = if pressed {
                    InputEvent::MouseDown { button }
                } else {
                    InputEvent::MouseUp { button }
                };
                let _ = input_sender.send(event);
                let _ = input_sender.send(InputEvent::PointerMove { x, y, pressed });
            }
        }
    }

    /// Forward a keyboard event to the WASM app
    pub async fn handle_keyboard(&self, session_id: &str, key: String, code: String, pressed: bool) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(input_sender) = &session.input_sender {
                let event = if pressed {
                    InputEvent::KeyDown { key, code }
                } else {
                    InputEvent::KeyUp { key, code }
                };
                let _ = input_sender.send(event);
            }
        }
    }

    pub async fn cleanup_session(&self, session_id: &str) -> Result<()> {
        info!("Cleaning up WASM session: {}", session_id);
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            session.cancel_token.cancel();
            info!("WASM session {} cancelled", session_id);
        }
        Ok(())
    }
}

/// Run the WASM module in a render loop, pushing RGBA frames to the channel
fn run_wasm_render_loop(
    wasm_path: &str,
    frame_tx: mpsc::Sender<Vec<u8>>,
    mut input_rx: mpsc::UnboundedReceiver<InputEvent>,
    cancel_token: CancellationToken,
    frame_interval: std::time::Duration,
    session_id: &str,
    width: u32,
    height: u32,
    framerate: u8,
) -> Result<()> {
    // Create wasmtime engine and store
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    // Load the WASM module
    let module = Module::from_file(&engine, wasm_path)
        .with_context(|| format!("Failed to load WASM module: {}", wasm_path))?;

    // Create a linker and define a host function for logging
    let mut linker = Linker::new(&engine);
    linker.func_wrap("env", "console_log", |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
        debug!(target: "wasm-log", "[wasm] console_log host function CALLED: ptr={}, len={}", ptr, len);
        let mem = match caller.get_export("memory") {
            Some(Extern::Memory(mem)) => mem,
            _ => {
                error!(target: "wasm-log", "[wasm] console_log: memory export not found");
                return;
            }
        };
        let mut buf = vec![0u8; len as usize];
        match mem.read(&mut caller, ptr as usize, &mut buf) {
            Ok(_) => {
                match std::str::from_utf8(&buf) {
                    Ok(msg) => debug!(target: "wasm-log", "[wasm] {}", msg),
                    Err(e) => error!(target: "wasm-log", "[wasm] console_log: utf8 error: {}", e),
                }
            }
            Err(e) => {
                error!(target: "wasm-log", "[wasm] console_log: memory read error: {}", e);
            }
        }
    })?;
    let instance = linker
        .instantiate(&mut store, &module)
        .context("Failed to instantiate WASM module")?;


    // Get exported functions
    let render_frame = instance
        .get_typed_func::<(), ()>(&mut store, "render_file_explorer_frame")
        .context("Missing export: render_file_explorer_frame")?;

    let get_fb_ptr = instance
        .get_typed_func::<(), i32>(&mut store, "get_framebuffer_ptr")
        .context("Missing export: get_framebuffer_ptr")?;

    let get_fb_size = instance
        .get_typed_func::<(), i32>(&mut store, "get_framebuffer_size")
        .context("Missing export: get_framebuffer_size")?;

    // Try to get exported setters for width/height/framerate if present
    let set_size = instance.get_typed_func::<(i32, i32), ()>(&mut store, "set_size").ok();
    let set_width = instance.get_typed_func::<i32, ()>(&mut store, "set_width").ok();
    let set_height = instance.get_typed_func::<i32, ()>(&mut store, "set_height").ok();
    let set_framerate = instance.get_typed_func::<i32, ()>(&mut store, "set_framerate").ok();

    // Get handle_pointer_event function for input forwarding
    let handle_pointer_event = instance
        .get_typed_func::<(f32, f32, u32), ()>(&mut store, "handle_pointer_event")
        .ok();

    // Get handle_key_event function for keyboard input
    let handle_key_event = instance
        .get_typed_func::<(u32, u32, u32), ()>(&mut store, "handle_key_event")
        .ok();

    // Set rendering parameters if the WASM module supports it
    if let Some(set_size) = &set_size {
        let _ = set_size.call(&mut store, (width as i32, height as i32));
    } else {
        if let Some(set_width) = &set_width {
            let _ = set_width.call(&mut store, width as i32);
        }
        if let Some(set_height) = &set_height {
            let _ = set_height.call(&mut store, height as i32);
        }
    }
    if let Some(set_framerate) = &set_framerate {
        let _ = set_framerate.call(&mut store, framerate as i32);
    }

    // Get the memory export
    let memory = instance
        .get_memory(&mut store, "memory")
        .context("Missing memory export")?;

    info!(
        "[session {}] WASM module loaded, starting render loop",
        session_id
    );

    let mut frame_count = 0u64;

    loop {
        if cancel_token.is_cancelled() {
            info!(
                "[session {}] Render loop cancelled after {} frames",
                session_id, frame_count
            );
            break;
        }

        let start = std::time::Instant::now();

        // Process all pending input events
        while let Ok(input_event) = input_rx.try_recv() {
            match input_event {
                InputEvent::PointerMove { x, y, pressed } => {
                    if let Some(ref handle_ptr) = handle_pointer_event {
                        let pressed_u32 = if pressed { 1 } else { 0 };
                        if let Err(e) = handle_ptr.call(&mut store, (x, y, pressed_u32)) {
                            warn!("[session {}] handle_pointer_event failed: {}", session_id, e);
                        }
                    }
                }
                InputEvent::MouseDown { button } => {
                    debug!("[session {}] Mouse button {} down", session_id, button);
                    // Button state is tracked and sent via PointerMove events
                }
                InputEvent::MouseUp { button } => {
                    debug!("[session {}] Mouse button {} up", session_id, button);
                    // Button state is tracked and sent via PointerMove events
                }
                InputEvent::KeyDown { key, code } => {
                    debug!("[session {}] Key down: {} ({})", session_id, key, code);
                    if let Some(ref handle_key) = handle_key_event {
                        // Allocate space in WASM memory for the key string
                        let key_bytes = key.as_bytes();
                        let mem_data = memory.data_mut(&mut store);

                        // Write key to a temporary location in memory (use offset 0 for simplicity)
                        // In production, you'd want proper memory management
                        if key_bytes.len() < 1000 && mem_data.len() > key_bytes.len() {
                            mem_data[0..key_bytes.len()].copy_from_slice(key_bytes);

                            if let Err(e) = handle_key.call(&mut store, (0, key_bytes.len() as u32, 1)) {
                                warn!("[session {}] handle_key_event failed: {}", session_id, e);
                            }
                        }
                    }
                }
                InputEvent::KeyUp { key, code } => {
                    debug!("[session {}] Key up: {} ({})", session_id, key, code);
                    if let Some(ref handle_key) = handle_key_event {
                        // Allocate space in WASM memory for the key string
                        let key_bytes = key.as_bytes();
                        let mem_data = memory.data_mut(&mut store);

                        // Write key to a temporary location in memory
                        if key_bytes.len() < 1000 && mem_data.len() > key_bytes.len() {
                            mem_data[0..key_bytes.len()].copy_from_slice(key_bytes);

                            if let Err(e) = handle_key.call(&mut store, (0, key_bytes.len() as u32, 0)) {
                                warn!("[session {}] handle_key_event failed: {}", session_id, e);
                            }
                        }
                    }
                }
            }
        }

        // Call render function
        if let Err(e) = render_frame.call(&mut store, ()) {
            error!(
                "[session {}] render_file_explorer_frame failed: {}",
                session_id, e
            );
            break;
        }

        // Read framebuffer pointer and size
        let fb_ptr = get_fb_ptr.call(&mut store, ()).unwrap_or(0) as usize;
        let fb_size = get_fb_size.call(&mut store, ()).unwrap_or(0) as usize;

        if fb_size == 0 || fb_ptr == 0 {
            warn!(
                "[session {}] Invalid framebuffer: ptr={}, size={}",
                session_id, fb_ptr, fb_size
            );
            std::thread::sleep(frame_interval);
            continue;
        }

        // Read RGBA data from WASM linear memory
        let mem_data = memory.data(&store);
        if fb_ptr + fb_size > mem_data.len() {
            warn!(
                "[session {}] Framebuffer out of bounds: ptr={}, size={}, mem_len={}",
                session_id,
                fb_ptr,
                fb_size,
                mem_data.len()
            );
            std::thread::sleep(frame_interval);
            continue;
        }


        let frame_data = mem_data[fb_ptr..fb_ptr + fb_size].to_vec();

        // Log the first 16 RGBA pixels for debugging
        if frame_count < 5 {
            let mut pixel_log = String::new();
            for i in 0..16.min(frame_data.len() / 4) {
                let r = frame_data[i * 4];
                let g = frame_data[i * 4 + 1];
                let b = frame_data[i * 4 + 2];
                let a = frame_data[i * 4 + 3];
                pixel_log.push_str(&format!("[{} {} {} {}] ", r, g, b, a));
            }
            tracing::debug!("[session {}] First 16 RGBA pixels: {}", session_id, pixel_log);
        }

        // Send frame to GStreamer pipeline
        if frame_tx.blocking_send(frame_data).is_err() {
            info!(
                "[session {}] Frame receiver dropped, stopping render loop",
                session_id
            );
            break;
        }

        frame_count += 1;
        if frame_count % 30 == 0 {
            tracing::debug!(
                "[session {}] Rendered {} frames",
                session_id,
                frame_count
            );
        }

        // Maintain target framerate
        let elapsed = start.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
    }

    Ok(())
}
