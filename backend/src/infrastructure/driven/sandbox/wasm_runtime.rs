use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use wasmtime::*;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 600;
const PIXEL_SIZE: u32 = 4; // RGBA8

/// Manages WASM application instances, replacing XvfbManager
pub struct WasmAppManager {
    sessions: Arc<RwLock<HashMap<String, WasmSession>>>,
    wasm_dir: String,
}

struct WasmSession {
    cancel_token: CancellationToken,
    frame_sender: Option<mpsc::Sender<Vec<u8>>>,
    width: u32,
    height: u32,
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

        let session = WasmSession {
            cancel_token: cancel_token.clone(),
            frame_sender: Some(frame_tx.clone()),
            width: width as u32,
            height: height as u32,
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
            let result = tokio::task::spawn_blocking(move || {
                run_wasm_render_loop(&wasm_path, frame_tx, token, frame_interval, &session_id_owned)
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
        _session_id: &str,
        _x: f32,
        _y: f32,
        _pressed: bool,
    ) {
        // Input forwarding requires calling into the WASM instance.
        // For now, pointer events are collected but the WASM instance
        // runs in a separate blocking thread. A proper implementation
        // would use a channel to send input events to the render loop.
        // This is a TODO for full input support.
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
    cancel_token: CancellationToken,
    frame_interval: std::time::Duration,
    session_id: &str,
) -> Result<()> {
    // Create wasmtime engine and store
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    // Load the WASM module
    let module = Module::from_file(&engine, wasm_path)
        .with_context(|| format!("Failed to load WASM module: {}", wasm_path))?;

    // Create a linker (no WASI needed for now — the module is self-contained)
    let linker = Linker::new(&engine);
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
            tracing::info!("[session {}] First 16 RGBA pixels: {}", session_id, pixel_log);
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
            tracing::info!(
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
