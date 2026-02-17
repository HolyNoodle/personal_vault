use anyhow::{Context, Result};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum AppMessage {
    Init { width: u32, height: u32, framerate: u32 },
    PointerMove { x: f32, y: f32 },
    PointerButton { x: f32, y: f32, button: u8, pressed: bool },
    KeyEvent { key: String, pressed: bool },
    Resize { width: u32, height: u32 },
    Shutdown,
}
use serde_json;
use std::collections::HashMap;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::info;


// ── Shared-framebuffer reader ────────────────────────────────────────────────

/// Safety wrapper so the read-only mmap pointer is Send across threads.
struct MmapReader {
    ptr: *const u8,
    size: usize,
}
unsafe impl Send for MmapReader {}

impl MmapReader {
    fn read_frame(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size).to_vec() }
    }
}

impl Drop for MmapReader {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
        }
    }
}

// ── Session state ────────────────────────────────────────────────────────────

struct NativeSession {
    cancel_token: CancellationToken,
    /// Channel to send AppMessages → the write-task → the child's control socket.
    input_sender: mpsc::UnboundedSender<AppMessage>,
    last_x: Arc<std::sync::Mutex<f32>>,
    last_y: Arc<std::sync::Mutex<f32>>,
    button_pressed: Arc<std::sync::Mutex<bool>>,
    child: Arc<std::sync::Mutex<std::process::Child>>,
}

// ── NativeAppManager ─────────────────────────────────────────────────────────

/// Replaces `WasmAppManager`. Spawns native child processes that communicate
/// with the backend via a memfd shared framebuffer + Unix socket control channel.
pub struct NativeAppManager {
    sessions: Arc<RwLock<HashMap<String, NativeSession>>>,
    apps_root: String,
}

impl NativeAppManager {
    pub fn new(apps_root: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            apps_root,
        }
    }

    /// Launch a native app for a session.
    ///
    /// Returns a channel receiver that produces raw RGBA frames at the requested
    /// framerate — the same type the GStreamer pipeline expects.
    pub async fn launch_app(
        &self,
        session_id: &str,
        app_name: &str,
        width: u16,
        height: u16,
        framerate: u8,
    ) -> Result<mpsc::Receiver<Vec<u8>>> {
        let binary_name = app_name.replace('-', "_");
        let binary_path = format!("{}/{}/{}", self.apps_root, binary_name, binary_name);

        info!(
            "[session {}] Launching native app '{}' from {} ({}x{}@{}fps)",
            session_id, app_name, binary_path, width, height, framerate
        );

        if !std::path::Path::new(&binary_path).exists() {
            return Err(anyhow::anyhow!("Native binary not found: {}", binary_path));
        }

        let w = width as u32;
        let h = height as u32;
        let fps = framerate as u32;
        let fb_size = (w * h * 4) as usize;

        // ── 1. Create memfd for the shared framebuffer ──────────────────────
        let memfd = unsafe {
            libc::memfd_create(
                b"sandbox-fb\0".as_ptr() as *const libc::c_char,
                libc::MFD_CLOEXEC,
            )
        };
        if memfd < 0 {
            return Err(anyhow::anyhow!(
                "memfd_create failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        if unsafe { libc::ftruncate(memfd, fb_size as libc::off_t) } < 0 {
            unsafe { libc::close(memfd) };
            return Err(anyhow::anyhow!(
                "ftruncate failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        // Dup a copy for the child (without CLOEXEC so it survives exec)
        let child_fb_fd = unsafe { libc::dup(memfd) };
        if child_fb_fd < 0 {
            unsafe { libc::close(memfd) };
            return Err(anyhow::anyhow!(
                "dup(memfd) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        clear_cloexec(child_fb_fd)?;

        // mmap read-only on the backend side
        let backend_fb_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                fb_size,
                libc::PROT_READ,
                libc::MAP_SHARED,
                memfd,
                0,
            )
        };
        unsafe { libc::close(memfd) }; // fd no longer needed; mapping persists
        if backend_fb_ptr == libc::MAP_FAILED {
            unsafe { libc::close(child_fb_fd) };
            return Err(anyhow::anyhow!(
                "mmap(memfd) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let reader = MmapReader {
            ptr: backend_fb_ptr as *const u8,
            size: fb_size,
        };

        // ── 2. Create Unix socketpair for the control channel ───────────────
        let mut sock_fds = [-1i32; 2];
        let rc = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
                0,
                sock_fds.as_mut_ptr(),
            )
        };
        if rc < 0 {
            unsafe { libc::close(child_fb_fd) };
            return Err(anyhow::anyhow!(
                "socketpair failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let child_ctrl_fd = sock_fds[0]; // app's end
        let backend_ctrl_fd = sock_fds[1]; // backend's end

        // Remove CLOEXEC from the child's ctrl fd so it survives exec
        clear_cloexec(child_ctrl_fd).map_err(|e| {
            unsafe {
                libc::close(child_fb_fd);
                libc::close(child_ctrl_fd);
                libc::close(backend_ctrl_fd);
            }
            e
        })?;

        // ── 3. Spawn the child process ──────────────────────────────────────
        let child = std::process::Command::new(&binary_path)
            .env("SANDBOX_FB_FD", child_fb_fd.to_string())
            .env("SANDBOX_CTRL_FD", child_ctrl_fd.to_string())
            .env("SANDBOX_WIDTH", w.to_string())
            .env("SANDBOX_HEIGHT", h.to_string())
            .env("SANDBOX_FRAMERATE", fps.to_string())
            .spawn()
            .with_context(|| format!("Failed to spawn {}", binary_path))?;

        // Close the child-side fds in the parent now that the child has them
        unsafe {
            libc::close(child_fb_fd);
            libc::close(child_ctrl_fd);
        }

        // ── 4. Set up the control-socket write task ─────────────────────────
        let backend_stream = unsafe { UnixStream::from_raw_fd(backend_ctrl_fd) };
        backend_stream.set_nonblocking(true).ok();
        let tokio_stream = tokio::net::UnixStream::from_std(backend_stream)
            .context("Failed to convert UnixStream to tokio")?;
        let (_, mut write_half) = tokio::io::split(tokio_stream);

        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<AppMessage>();

        // Send the Init message first
        send_message_to_half(
            &mut write_half,
            &AppMessage::Init { width: w, height: h, framerate: fps },
        )
        .await
        .ok();

        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(msg) = input_rx.recv().await {
                if let Ok(json) = serde_json::to_vec(&msg) {
                    let len = (json.len() as u32).to_le_bytes();
                    if write_half.write_all(&len).await.is_err() {
                        break;
                    }
                    if write_half.write_all(&json).await.is_err() {
                        break;
                    }
                }
            }
        });

        // ── 5. Frame-read task: copy from mmap → GStreamer channel ──────────
        let cancel_token = CancellationToken::new();
        let (frame_tx, frame_rx) = mpsc::channel::<Vec<u8>>(2);

        let frame_interval = std::time::Duration::from_millis(
            (1000u64 / fps.max(1) as u64).max(16),
        );
        let cancel_clone = cancel_token.clone();
        let session_str = session_id.to_string();
        let frame_tx_clone = frame_tx.clone();

        tokio::spawn(async move {
            let mut frame_count = 0u64;
            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        info!("[session {}] Frame-read task cancelled after {} frames",
                            session_str, frame_count);
                        break;
                    }
                    _ = tokio::time::sleep(frame_interval) => {
                        let frame = reader.read_frame();
                        if frame_tx_clone.send(frame).await.is_err() {
                            info!("[session {}] Frame receiver dropped, stopping",
                                session_str);
                            break;
                        }
                        frame_count += 1;
                        if frame_count % 30 == 0 {
                            tracing::debug!("[session {}] {} frames sent", session_str, frame_count);
                        }
                    }
                }
            }
            // reader Drop unmaps the memory
        });

        // ── 6. Store session state ──────────────────────────────────────────
        let session = NativeSession {
            cancel_token,
            input_sender: input_tx,
            last_x: Arc::new(std::sync::Mutex::new(0.0)),
            last_y: Arc::new(std::sync::Mutex::new(0.0)),
            button_pressed: Arc::new(std::sync::Mutex::new(false)),
            child: Arc::new(std::sync::Mutex::new(child)),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session);

        Ok(frame_rx)
    }

    pub async fn handle_pointer_event(
        &self,
        session_id: &str,
        x: f32,
        y: f32,
        _pressed: bool,
    ) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            *session.last_x.lock().unwrap() = x;
            *session.last_y.lock().unwrap() = y;
            let _ = session.input_sender.send(AppMessage::PointerMove { x, y });
        }
    }

    pub async fn handle_mouse_button(&self, session_id: &str, button: u8, pressed: bool) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            *session.button_pressed.lock().unwrap() = pressed;
            let x = *session.last_x.lock().unwrap();
            let y = *session.last_y.lock().unwrap();
            let _ = session.input_sender.send(AppMessage::PointerButton { x, y, button, pressed });
        }
    }

    pub async fn handle_keyboard(
        &self,
        session_id: &str,
        key: String,
        _code: String,
        pressed: bool,
    ) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let _ = session.input_sender.send(AppMessage::KeyEvent { key, pressed });
        }
    }

    pub async fn cleanup_session(&self, session_id: &str) -> Result<()> {
        info!("Cleaning up native session: {}", session_id);
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            // Ask the app to shut down gracefully
            let _ = session.input_sender.send(AppMessage::Shutdown);
            session.cancel_token.cancel();

            // Kill the child process
            if let Ok(mut child) = session.child.lock() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        Ok(())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Clear the CLOEXEC flag on a raw file descriptor.
fn clear_cloexec(fd: i32) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(anyhow::anyhow!(
            "fcntl(F_GETFD, {}) failed: {}",
            fd,
            std::io::Error::last_os_error()
        ));
    }
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) };
    if rc < 0 {
        return Err(anyhow::anyhow!(
            "fcntl(F_SETFD, {}) failed: {}",
            fd,
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// Write one AppMessage to an AsyncWrite split half.
async fn send_message_to_half<W>(writer: &mut W, msg: &AppMessage) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;
    let json = serde_json::to_vec(msg)?;
    let len = (json.len() as u32).to_le_bytes();
    writer.write_all(&len).await?;
    writer.write_all(&json).await?;
    Ok(())
}
