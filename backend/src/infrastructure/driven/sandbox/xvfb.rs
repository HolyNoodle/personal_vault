use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use x11rb::protocol::xproto::ConnectionExt;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::AtomicU16;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{error, info, warn, debug};
use x11rb::connection::Connection;
use x11rb::protocol::xtest::ConnectionExt as XTestExt;
use x11rb::rust_connection::RustConnection;

use super::gstreamer::GStreamerManager;

pub struct XvfbManager {
    displays: Arc<RwLock<HashMap<String, XvfbSession>>>,
    apps_root: String,
    next_display: Arc<AtomicU16>,
}

struct XvfbSession {
    display_str: String,
    process: Option<Child>,
    app_process: Option<Child>,
    x11_conn: Option<Arc<RustConnection>>,
    keysym_map: Arc<HashMap<u32, (u8, bool)>>,
    shift_keycode: u8,
    gst_pipeline: Option<gst::Pipeline>,
}

impl XvfbManager {
    pub fn new(apps_root: String) -> Self {
        Self {
            displays: Arc::new(RwLock::new(HashMap::new())),
            apps_root,
            next_display: Arc::new(AtomicU16::new(0)),
        }
    }

    fn alloc_display(&self) -> u16 {
        // Always return 100 for now; can be improved if multi-display needed
        100 + self.next_display.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }


    pub async fn start_xvfb(&self, session_id: &str, width: u16, height: u16) -> Result<(u16, String)> {
        let display_number = self.alloc_display();
        let display_str = format!(":{}", display_number);
        let resolution = format!("{}x{}x24", width, height);


        debug!("About to spawn Xvfb process for session {} on {} ({}x{})", session_id, display_str, width, height);
        let xvfb_child = unsafe {
            Command::new("Xvfb")
            .arg(&display_str)
            .arg("-screen")
            .arg("0")
            .arg(&resolution)
            .arg("-ac")
            .arg("+extension")
            .arg("GLX")
            .arg("+extension")
            .arg("XTEST")
            .arg("+render")
            .arg("-noreset")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()
                .context("Failed to start Xvfb")?
        };

        debug!("Xvfb process spawned for session {}", session_id);

        // Give Xvfb time to initialize
        debug!("Sleeping 500ms to let Xvfb initialize for session {}", session_id);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        debug!("Done sleeping, about to connect to Xvfb with x11rb for session {}", session_id);

        // Hide the mouse cursor by setting a blank cursor using xsetroot
        // This will affect all windows on this Xvfb display
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg("xsetroot -cursor_name blank")
            .env("DISPLAY", &display_str)
            .status();
        match status {
            Ok(s) if s.success() => debug!("xsetroot -cursor_name blank succeeded for session {}", session_id),
            Ok(s) => warn!("xsetroot -cursor_name blank failed with status {:?} for session {}", s.code(), session_id),
            Err(e) => warn!("Failed to run xsetroot -cursor_name blank for session {}: {}", session_id, e),
        }

        // Connect to Xvfb via x11rb and build keysym→keycode map
        let session_id_owned = session_id.to_string();
        let display_str_clone = display_str.clone();

        let (conn, keysym_map, shift_keycode) =
            tokio::task::spawn_blocking(move || -> Result<(Arc<RustConnection>, Arc<HashMap<u32, (u8, bool)>>, u8)> {
                debug!("In spawn_blocking: connecting to Xvfb display {} for session {}", display_str_clone, session_id_owned);
                let (conn, _screen_num) = RustConnection::connect(Some(&display_str_clone))
                    .context("Failed to connect to Xvfb display")?;

                debug!("Connected to Xvfb display {} for session {}", display_str_clone, session_id_owned);

                let setup = conn.setup();
                let min_kc = setup.min_keycode;
                let max_kc = setup.max_keycode;

                let map = conn
                    .get_keyboard_mapping(min_kc, max_kc - min_kc + 1)?
                    .reply()
                    .context("Failed to get keyboard mapping")?;

                let syms_per = map.keysyms_per_keycode as usize;
                let mut keysym_map: HashMap<u32, (u8, bool)> = HashMap::new();
                for (i, chunk) in map.keysyms.chunks(syms_per).enumerate() {
                    let kc = min_kc + i as u8;
                    if let Some(&sym) = chunk.first() {
                        if sym != 0 {
                            keysym_map.entry(sym).or_insert((kc, false));
                        }
                    }
                    if let Some(&sym) = chunk.get(1) {
                        if sym != 0 {
                            keysym_map.entry(sym).or_insert((kc, true));
                        }
                    }
                }

                // Left Shift keysym = 0xFFE1
                let shift_keycode = keysym_map.get(&0xFFE1).map(|&(kc, _)| kc).unwrap_or(50);

                debug!("Built keysym map and shift_keycode for session {}", session_id_owned);
                Ok((Arc::new(conn), Arc::new(keysym_map), shift_keycode))
            })
            .await
            .context("spawn_blocking panicked")??;

        debug!(
            "x11rb connected to {} for session {} (shift_keycode={})",
            display_str, session_id, shift_keycode
        );

        let session = XvfbSession {
            display_str: display_str.clone(),
            process: Some(xvfb_child),
            app_process: None,
            x11_conn: Some(conn),
            keysym_map,
            shift_keycode,
            gst_pipeline: None,
        };

        let mut displays = self.displays.write().await;
        displays.insert(session_id.to_string(), session);

        Ok((display_number, display_str))
    }

    pub async fn launch_app(
        &self,
        session_id: &str,
        app_name: &str,
        width: u16,
        height: u16,
    ) -> Result<()> {
        let binary_name = app_name.replace('-', "_");
        let binary_path = format!("{}/{}/{}", self.apps_root, binary_name, binary_name);

        debug!("launch_app: about to read display_str for session {}", session_id);
        let display_str = {
            let displays = self.displays.read().await;
            displays
                .get(session_id)
                .map(|s| s.display_str.clone())
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?
        };
        debug!("launch_app: got display_str, about to spawn app for session {}", session_id);

        let ipc_socket_path = std::env::var("IPC_SOCKET_PATH")
            .unwrap_or_else(|_| "/tmp/sandbox-ipc.sock".to_string());

        let mut child = unsafe {
            Command::new(&binary_path)
                .env("DISPLAY", &display_str)
                .env("IPC_SOCKET_PATH", &ipc_socket_path)
                .env("SANDBOX_WIDTH", width.to_string())
                .env("SANDBOX_HEIGHT", height.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()
                .with_context(|| format!("Failed to spawn {}", binary_path))?
        };
        debug!("App process spawned for session {}: pid={:?}", session_id, child.id());

        if let Some(stdout) = child.stdout.take() {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let app = app_name.to_string();
            tokio::spawn(async move {
                let mut lines = reader.lines();
                loop {
                    match lines.next_line().await {
                        Ok(Some(line)) => {
                            info!("App stdout [{}]: {}", app, line);
                        },
                        Ok(None) => break,
                        Err(e) => { error!("App stdout [{}] read error: {}", app, e); break; }
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            let app = app_name.to_string();
            tokio::spawn(async move {
                let mut lines = reader.lines();
                loop {
                    match lines.next_line().await {
                        Ok(Some(line)) => {
                            error!("App stderr [{}]: {}", app, line);
                        },
                        Ok(None) => break,
                        Err(e) => { error!("App stderr [{}] read error: {}", app, e); break; }
                    }
                }
            });
        }

        debug!("launch_app: about to write app_process for session {}", session_id);
        let mut displays = self.displays.write().await;
        if let Some(session) = displays.get_mut(session_id) {
            session.app_process = Some(child);
        } else {
            warn!("Session not found when storing app_process for {}", session_id);
        }
        debug!("launch_app: completed for session {}", session_id);
        Ok(())
    }

    pub async fn start_capture(
        &self,
        session_id: &str,
        framerate: u8,
        gstreamer: &GStreamerManager,
    ) -> Result<std::sync::mpsc::Receiver<Vec<u8>>> {
        let display_str = {
            let displays = self.displays.read().await;
            displays
                .get(session_id)
                .map(|s| s.display_str.clone())
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?
        };

        let (pipeline, rx) =
            gstreamer.start_ximagesrc_pipeline(session_id, &display_str, framerate)?;

        let mut displays = self.displays.write().await;
        if let Some(session) = displays.get_mut(session_id) {
            session.gst_pipeline = Some(pipeline);
        }

        Ok(rx)
    }

    pub async fn handle_mouse_move(&self, session_id: &str, x: i32, y: i32) {
        let conn = {
            let displays = self.displays.read().await;
            displays.get(session_id).and_then(|s| s.x11_conn.clone())
        };
        if let Some(conn) = conn {
            // MOTION_NOTIFY event type = 6
            if let Err(e) = (&*conn)
                .xtest_fake_input(6, 0, 0, 0u32, x as i16, y as i16, 0)
                .and_then(|_| (&*conn).flush())
            {
                warn!("handle_mouse_move x11rb error: {}", e);
            }
        }
    }

    pub async fn handle_mouse_button(&self, session_id: &str, button: u8, pressed: bool) {
        let conn = {
            let displays = self.displays.read().await;
            displays.get(session_id).and_then(|s| s.x11_conn.clone())
        };
        if let Some(conn) = conn {
            // BUTTON_PRESS_EVENT = 4, BUTTON_RELEASE_EVENT = 5
            let type_ = if pressed { 4u8 } else { 5u8 };
            if let Err(e) = (&*conn)
                .xtest_fake_input(type_, button, 0, 0u32, 0, 0, 0)
                .and_then(|_| (&*conn).flush())
            {
                warn!("handle_mouse_button x11rb error: {}", e);
            }
        }
    }

    pub async fn handle_keyboard(&self, session_id: &str, key: &str, pressed: bool) {
        let (conn, keysym_map, shift_keycode) = {
            let displays = self.displays.read().await;
            match displays.get(session_id) {
                Some(s) => match &s.x11_conn {
                    Some(c) => (c.clone(), s.keysym_map.clone(), s.shift_keycode),
                    None => return,
                },
                None => return,
            }
        };

        let Some(keysym) = browser_key_to_keysym(key) else { return };
        let Some(&(keycode, needs_shift)) = keysym_map.get(&keysym) else { return };

        // KEY_PRESS_EVENT = 2, KEY_RELEASE_EVENT = 3
        let result: Result<(), x11rb::errors::ConnectionError> = (|| {
            if pressed {
                if needs_shift {
                    (&*conn).xtest_fake_input(2, shift_keycode, 0, 0u32, 0, 0, 0)?;
                }
                (&*conn).xtest_fake_input(2, keycode, 0, 0u32, 0, 0, 0)?;
            } else {
                (&*conn).xtest_fake_input(3, keycode, 0, 0u32, 0, 0, 0)?;
                if needs_shift {
                    (&*conn).xtest_fake_input(3, shift_keycode, 0, 0u32, 0, 0, 0)?;
                }
            }
            (&*conn).flush()?;
            Ok(())
        })();

        if let Err(e) = result {
            warn!("handle_keyboard x11rb error: {}", e);
        }
    }

    pub async fn cleanup_session(&self, session_id: &str) -> Result<()> {
        info!("cleanup_session called for session {}", session_id);
        let mut displays = self.displays.write().await;
        if let Some(mut session) = displays.remove(session_id) {
            // Stop GStreamer pipeline
            if let Some(pipeline) = session.gst_pipeline.take() {
                info!("Stopping GStreamer pipeline for session {}", session_id);
                let _ = pipeline.set_state(gst::State::Null);
            }

            // Drop x11 connection
            info!("Dropping x11 connection for session {}", session_id);
            drop(session.x11_conn.take());

            // Kill app process
            if let Some(mut child) = session.app_process.take() {
                info!("Killing app process for session {}", session_id);
                kill_child(&mut child, "app").await;
            }

            // Kill Xvfb
            if let Some(mut child) = session.process.take() {
                info!("Killing Xvfb process for session {}", session_id);
                kill_child(&mut child, "xvfb").await;
            }
        } else {
            info!("No session found for cleanup: {}", session_id);
        }
        info!("cleanup_session finished for session {}", session_id);
        Ok(())
    }
}

/// Map browser key names to X11 keysyms.
fn browser_key_to_keysym(key: &str) -> Option<u32> {
    match key {
        "Enter" => return Some(0xFF0D),
        "Backspace" => return Some(0xFF08),
        "Tab" => return Some(0xFF09),
        "Escape" => return Some(0xFF1B),
        "Delete" => return Some(0xFFFF),
        "ArrowLeft" => return Some(0xFF51),
        "ArrowUp" => return Some(0xFF52),
        "ArrowRight" => return Some(0xFF53),
        "ArrowDown" => return Some(0xFF54),
        " " => return Some(0x0020),
        _ => {}
    }
    // Single printable ASCII char: keysym == Unicode codepoint
    let c = key.chars().next()?;
    if c.is_ascii() && !c.is_control() {
        Some(c as u32)
    } else {
        None
    }
}

async fn kill_child(child: &mut Child, label: &str) {
    #[cfg(unix)]
    if let Some(id) = child.id() {
        unsafe {
            use libc::{killpg, SIGTERM};
            let pgid = libc::getpgid(id as libc::pid_t);
            if pgid > 0 {
                let res = killpg(pgid, SIGTERM);
                if res == 0 {
                    info!("Sent SIGTERM to process group {} for {} (pid={})", pgid, label, id);
                } else {
                    let err = std::io::Error::last_os_error();
                    error!("Failed to send SIGTERM to process group {} for {} (pid={}): {}", pgid, label, id, err);
                }
            } else {
                error!("Failed to get pgid for {} (pid={})", label, id);
            }
        }
    }
    match child.kill().await {
        Ok(_) => info!("kill() succeeded for {}", label),
        Err(e) => error!("kill() failed for {}: {}", label, e),
    }
    match child.wait().await {
        Ok(status) => info!("wait() for {} returned: {:?}", label, status),
        Err(e) => error!("wait() failed for {}: {}", label, e),
    }
    info!("{} process cleaned up", label);
}
