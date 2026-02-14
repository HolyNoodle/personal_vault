use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, error, info};
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
        let mut displays = self.displays.write().await;
        if let Some(mut session) = displays.remove(session_id) {
            info!("Stopping Xvfb for session {}", session_id);
            
            // Kill application process first
            if let Some(mut child) = session.app_process.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            
            // Kill window manager
            if let Some(mut child) = session.wm_process.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            
            // Then kill Xvfb
            if let Some(mut child) = session.process.take() {
                if let Err(e) = child.kill().await {
                    error!("Failed to kill Xvfb process: {}", e);
                }
                let _ = child.wait().await;
            }
        }
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
            
            // Store the window manager process
            let mut displays = self.displays.write().await;
            if let Some(session) = displays.get_mut(session_id) {
                session.wm_process = Some(wm_process);
            }
            drop(displays);
            
            // Give window manager time to start
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        // Build command based on application type
        let mut cmd = Command::new(command);
        cmd.env("DISPLAY", display_str);
        cmd.env("DBUS_SESSION_BUS_ADDRESS", &dbus_address);
        
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
                // Thunar doesn't need special args, just open to home directory
                cmd.arg("/root");
            },
            _ => {
                // For other apps, just run them as-is
            }
        }
        
        let child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context(format!("Failed to launch {}. Make sure it's installed.", command))?;

        // Store the child process to keep it alive
        let mut displays = self.displays.write().await;
        if let Some(session) = displays.get_mut(session_id) {
            session.app_process = Some(child);
        }
        drop(displays);

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
