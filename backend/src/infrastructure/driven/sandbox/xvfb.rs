use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
// ...existing code...
// Removed import for deleted SandboxPort trait

/// Manages Xvfb (X Virtual Framebuffer) instances
pub struct XvfbManager {
    displays: Arc<RwLock<HashMap<String, XvfbSession>>>,
}

struct XvfbSession {
    display_number: u16,
    process: Option<Child>,
    dbus_address: Option<String>,
    app_process: Option<Child>,
    wm_process: Option<Child>,
}

impl XvfbManager {
        /// Get the display string for a session, e.g. ":100"
        pub async fn get_display_str(&self, session_id: &str) -> Option<String> {
            let displays = self.displays.read().await;
            displays.get(session_id).map(|s| format!(":{}", s.display_number))
        }
    pub fn new() -> Self {
        Self {
            displays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_xvfb(&self, session_id: &str, width: u16, height: u16) -> Result<(u16, String, String)> {
        // Generate a unique display number for session isolation
        let display_number = 100 + (rand::random::<u16>() % 100);
        let display_str = format!(":{}", display_number);
        let resolution = format!("{}x{}x24", width, height);

        info!(
            "Starting Xvfb on display {} for session {} ({}x{})",
            display_str, session_id, width, height
        );

        // Start D-Bus session for this display
        let dbus_output = Command::new("dbus-launch")
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to start dbus-launch")?
            .wait_with_output()
            .await
            .context("Failed to wait for dbus-launch")?;

        let dbus_info = String::from_utf8_lossy(&dbus_output.stdout);
        let dbus_address = dbus_info
            .lines()
            .find(|line| line.starts_with("DBUS_SESSION_BUS_ADDRESS="))
            .and_then(|line| line.split('=').nth(1))
            .unwrap_or("unix:path=/run/user/0/bus")
            .to_string();

        info!("D-Bus session started: {}", dbus_address);

        // Start session-specific Xvfb
        let child = Command::new("Xvfb")
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
            .spawn()
            .context("Failed to start Xvfb. Make sure Xvfb is installed.")?;

        info!("Session-specific Xvfb started on display {} for session {}", display_str, session_id);

        // Give Xvfb time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let session = XvfbSession {
            display_number,
            process: Some(child),
            dbus_address: Some(dbus_address.clone()),
            app_process: None,
            wm_process: None,
        };

        let mut displays = self.displays.write().await;
        displays.insert(session_id.to_string(), session);

        Ok((display_number, display_str, dbus_address))
    }

    async fn cleanup_session(&self, session_id: &str) -> Result<()> {
        info!("cleanup_session called for session {}", session_id);
        let mut displays = self.displays.write().await;
        info!("displays before removal: keys={:?}", displays.keys().collect::<Vec<_>>());
        if let Some(mut session) = displays.remove(session_id) {
            info!("Session found for cleanup: display_number={}, app_process_present={}, wm_process_present={}, xvfb_process_present={}",
                session.display_number,
                session.app_process.is_some(),
                session.wm_process.is_some(),
                session.process.is_some());
            debug!("app_process state for session {}: {:?}", session_id, session.app_process);
            info!("Stopping Xvfb for session {}", session_id);

            // Helper to kill a process and its children
            async fn kill_process_and_children(child: &mut Child, label: &str) {
                if let Some(id) = child.id() {
                    // Send SIGTERM to the process group
                    #[cfg(unix)]
                    unsafe {
                        use libc::{killpg, SIGTERM};
                        let pgid = libc::getpgid(id as libc::pid_t);
                        if pgid > 0 {
                            let res = killpg(pgid, SIGTERM);
                            if res == 0 {
                                info!("Sent SIGTERM to {} process group {}", label, pgid);
                            } else {
                                error!("Failed to send SIGTERM to {} process group {}", label, pgid);
                            }
                        }
                    }
                }
                let _ = child.kill().await;
                let _ = child.wait().await;
            }

            // Kill application process first
            if let Some(mut child) = session.app_process.take() {
                info!("Cleaning up file-explorer process for session {} (pid={:?})", session_id, child.id());
                kill_process_and_children(&mut child, "app").await;
                info!("file-explorer process cleanup complete for session {}", session_id);
            } else {
                info!("No app_process found for session {} during cleanup", session_id);
            }

            // Kill window manager
            if let Some(mut child) = session.wm_process.take() {
                info!("Cleaning up window manager for session {} (pid={:?})", session_id, child.id());
                kill_process_and_children(&mut child, "wm").await;
                info!("window manager cleanup complete for session {}", session_id);
            } else {
                info!("No wm_process found for session {} during cleanup", session_id);
            }

            // Then kill Xvfb
            if let Some(mut child) = session.process.take() {
                info!("Cleaning up Xvfb for session {} (pid={:?})", session_id, child.id());
                kill_process_and_children(&mut child, "xvfb").await;
                info!("Xvfb cleanup complete for session {}", session_id);
            } else {
                info!("No Xvfb process found for session {} during cleanup", session_id);
            }
        } else {
            info!("No session found for cleanup for session_id={}", session_id);
        }
        info!("cleanup_session finished for session {}", session_id);
        Ok(())
    }

    pub async fn launch_app(&self, session_id: &str, display_str: &str, command: &str, width: u16, height: u16) -> Result<()> {
        debug!("Launching {} for session {} ({}x{})", command, session_id, width, height);

        // Special case: file-explorer as WASM app (no Xvfb, no openbox)
        if command == "file-explorer" {
            // Launch with wasmtime and the WASM file
            let wasm_path = "apps/file-explorer/target/wasm32-unknown-unknown/debug/file_explorer.wasm";
            let mut cmd = Command::new("wasmtime");
            cmd.arg(wasm_path);
            // Pass IPC socket path if needed
            if let Ok(ipc_path) = std::env::var("IPC_SOCKET_PATH") {
                cmd.env("IPC_SOCKET_PATH", ipc_path);
            }
            // Add any other required env vars or args here
            let child = cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            match child {
                Ok(mut child) => {
                    info!("Spawned WASM app process for session {}: pid={:?}", session_id, child.id());
                    // Log child process output for debugging
                    if let Some(stdout) = child.stdout.take() {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stdout);
                        let command_owned = command.to_string();
                        tokio::spawn(async move {
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                tracing::info!("App stdout [{}]: {}", command_owned, line);
                            }
                        });
                    }
                    if let Some(stderr) = child.stderr.take() {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stderr);
                        let command_owned = command.to_string();
                        tokio::spawn(async move {
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                tracing::error!("App stderr [{}]: {}", command_owned, line);
                            }
                        });
                    }
                    // Store the child process to keep it alive
                    let mut displays = self.displays.write().await;
                    if let Some(session) = displays.get_mut(session_id) {
                        info!("Storing app_process for session {}: pid={:?}", session_id, child.id());
                        session.app_process = Some(child);
                    } else {
                        warn!("Session not found when storing app_process for session {}", session_id);
                    }
                    drop(displays);
                }
                Err(e) => {
                    error!("Failed to launch WASM app '{}' for session {}: {}", command, session_id, e);
                    tracing::error!("App launch error [{}]: {}", command, e);
                    return Err(anyhow::anyhow!("Failed to launch {}: {}", command, e));
                }
            }
            return Ok(());
        }

        // Default: legacy/native apps (Xvfb, openbox, etc.)
        // ...existing code for other apps...
        Ok(())
    }
}

// Removed trait implementation for deleted SandboxPort
