use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use crate::domain::aggregates::VideoSessionId;
use crate::application::ports::SandboxPort;

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
    pub fn new() -> Self {
        Self {
            displays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn start_xvfb(&self, session_id: &str, width: u16, height: u16) -> Result<(u16, String, String)> {
        // Generate a random display number between 100-199
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

        // Start Xvfb
        let child = Command::new("Xvfb")
            .arg(&display_str)
            .arg("-screen")
            .arg("0")
            .arg(&resolution)
            .arg("-ac") // Disable access control
            .arg("+extension")
            .arg("GLX")
            .arg("+extension")
            .arg("XTEST") // Enable XTEST for input injection
            .arg("+render")
            .arg("-noreset")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Xvfb. Make sure Xvfb is installed.")?;

        info!("Xvfb started on display {} for session {}", display_str, session_id);

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

    async fn launch_app(&self, session_id: &str, display_str: &str, command: &str, width: u16, height: u16) -> Result<()> {
        debug!("Launching {} on display {} for session {} ({}x{})", command, display_str, session_id, width, height);

        // Get D-Bus address from session
        let dbus_address = {
            let displays = self.displays.read().await;
            displays
                .get(session_id)
                .and_then(|s| s.dbus_address.clone())
                .unwrap_or_else(|| "unix:path=/run/user/0/bus".to_string())
        };
        // Start window manager for GUI applications (not for terminals)
        if command != "xterm" {
            info!("Starting openbox window manager on display {}", display_str);
            let wm_process = Command::new("openbox")
                .env("DISPLAY", display_str)
                .env("DBUS_SESSION_BUS_ADDRESS", &dbus_address)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Failed to start openbox window manager")?;
            let mut displays = self.displays.write().await;
            if let Some(session) = displays.get_mut(session_id) {
                session.wm_process = Some(wm_process);
            }
            drop(displays);
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        // Build command based on application type
        let command_path = match command {
            "file-explorer" => "/app/target/release/file-explorer",
            _ => command,
        };
        
        let mut cmd = Command::new(command_path);
        cmd.env("DISPLAY", display_str);
        cmd.env("DBUS_SESSION_BUS_ADDRESS", &dbus_address);
        
        // Pass IPC socket path to file-explorer
        if command == "file-explorer" {
            if let Ok(ipc_path) = std::env::var("IPC_SOCKET_PATH") {
                cmd.env("IPC_SOCKET_PATH", ipc_path);
            }
        }
        
        // Application-specific arguments
        match command {
            "xterm" => {
                // Calculate terminal geometry (columns x rows based on pixel dimensions)
                // Standard terminal font: ~9 pixels wide, ~16 pixels tall
                let cols = width / 9;
                let rows = height / 16;
                let geometry = format!("{}x{}+0+0", cols, rows);
                cmd.args(&["-geometry", &geometry, "-maximized", "-e", "top"]);
            },
            "thunar" => {
                // Open file manager to storage directory
                cmd.arg("/data/storage");
            },
            "file-explorer" => {
                // Custom Rust file explorer app - uses IPC for communication
                // TODO: Implement IPC socket setup
            },
            _ => {
                // For other apps, just run them as-is
            }
        }
        
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
match child {
    Ok(mut child) => {
        info!("Spawned app process for session {}: command={}, pid={:?}", session_id, command, child.id());
        // Log child process output for debugging
        if let Some(stdout) = child.stdout.take() {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            tokio::spawn(async move {
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::info!("App stdout: {}", line);
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            tokio::spawn(async move {
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::error!("App stderr: {}", line);
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
        error!("Failed to launch {} for session {}: {}", command, session_id, e);
        return Err(anyhow::anyhow!("Failed to launch {}: {}", command, e));
    }
}

        // Maximize GUI applications after a short delay
        if command != "xterm" {
            let display = display_str.to_string();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                // Use wmctrl to maximize the window
                let _ = Command::new("wmctrl")
                    .arg("-r")
                    .arg(":ACTIVE:")
                    .arg("-b")
                    .arg("add,maximized_vert,maximized_horz")
                    .env("DISPLAY", &display)
                    .output()
                    .await;
            });
        }

        Ok(())
    }
}

impl SandboxPort for XvfbManager {
    async fn create_display(&self, session_id: &VideoSessionId, width: u16, height: u16) -> Result<String> {
        let (_display_num, display, _dbus_address) = self.start_xvfb(session_id.as_str(), width, height).await?;
        Ok(display)
    }

    async fn launch_application(&self, session_id: &VideoSessionId, display: &str, app: &str, width: u16, height: u16) -> Result<()> {
        self.launch_app(session_id.as_str(), display, app, width, height).await
    }

    async fn cleanup(&self, session_id: &VideoSessionId) -> Result<()> {
        self.cleanup_session(session_id.as_str()).await
    }
}
