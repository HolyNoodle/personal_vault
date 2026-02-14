use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
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
}

impl XvfbManager {
    pub fn new() -> Self {
        Self {
            displays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn start_xvfb(&self, session_id: &str, width: u16, height: u16) -> Result<(u16, String)> {
        // Generate a random display number between 100-199
        let display_number = 100 + (rand::random::<u16>() % 100);
        let display_str = format!(":{}", display_number);
        let resolution = format!("{}x{}x24", width, height);

        info!(
            "Starting Xvfb on display {} for session {} ({}x{})",
            display_str, session_id, width, height
        );

        // Start Xvfb
        let child = Command::new("Xvfb")
            .arg(&display_str)
            .arg("-screen")
            .arg("0")
            .arg(&resolution)
            .arg("-ac") // Disable access control
            .arg("+extension")
            .arg("GLX")
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
        };

        let mut displays = self.displays.write().await;
        displays.insert(session_id.to_string(), session);

        Ok((display_number, display_str))
    }

    async fn cleanup_session(&self, session_id: &str) -> Result<()> {
        let mut displays = self.displays.write().await;
        if let Some(mut session) = displays.remove(session_id) {
            info!("Stopping Xvfb for session {}", session_id);
            if let Some(mut child) = session.process.take() {
                if let Err(e) = child.kill() {
                    error!("Failed to kill Xvfb process: {}", e);
                }
                let _ = child.wait();
            }
        }
        Ok(())
    }

    async fn launch_app(&self, session_id: &str, display_str: &str, command: &str) -> Result<()> {
        debug!("Launching {} on display {} for session {}", command, display_str, session_id);

        // Launch application with environment
        let _child = Command::new(command)
            .args(&["-e", "top"]) // For xterm, show top command
            .env("DISPLAY", display_str)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context(format!("Failed to launch {}. Make sure it's installed.", command))?;

        Ok(())
    }
}

impl SandboxPort for XvfbManager {
    async fn create_display(&self, session_id: &VideoSessionId, width: u16, height: u16) -> Result<String> {
        let (_display_num, display) = self.start_xvfb(session_id.as_str(), width, height).await?;
        Ok(display)
    }

    async fn launch_application(&self, session_id: &VideoSessionId, display: &str, app: &str) -> Result<()> {
        self.launch_app(session_id.as_str(), display, app).await
    }

    async fn cleanup(&self, session_id: &VideoSessionId) -> Result<()> {
        self.cleanup_session(session_id.as_str()).await
    }
}
