use anyhow::{Context, Result};
use tokio::process::Command;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Manages input forwarding to X11 displays using xdotool
pub struct XdotoolInputManager {
    displays: Arc<RwLock<HashMap<String, String>>>,
}

impl XdotoolInputManager {
    pub fn new() -> Self {
        Self {
            displays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_session(&self, session_id: String, display: String) {
        let mut displays = self.displays.write().await;
        displays.insert(session_id, display);
    }

    pub async fn unregister_session(&self, session_id: &str) {
        let mut displays = self.displays.write().await;
        displays.remove(session_id);
    }

    pub async fn handle_mouse_move(&self, session_id: &str, x: i32, y: i32) -> Result<()> {
        let display = {
            let displays = self.displays.read().await;
            displays.get(session_id).cloned()
        };

        if let Some(display) = display {
            let output = Command::new("xdotool")
                .arg("mousemove")
                .arg("--sync")
                .arg(x.to_string())
                .arg(y.to_string())
                .env("DISPLAY", &display)
                .output()
                .await
                .context("Failed to execute xdotool mousemove")?;

            if !output.status.success() {
                error!("xdotool mousemove failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Ok(())
    }

    pub async fn handle_mouse_down(&self, session_id: &str, button: u8) -> Result<()> {
        let display = {
            let displays = self.displays.read().await;
            displays.get(session_id).cloned()
        };

        if let Some(display) = display {
            let output = Command::new("xdotool")
                .arg("mousedown")
                .arg(button.to_string())
                .env("DISPLAY", &display)
                .output()
                .await
                .context("Failed to execute xdotool mousedown")?;

            if !output.status.success() {
                error!("xdotool mousedown failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Ok(())
    }

    pub async fn handle_mouse_up(&self, session_id: &str, button: u8) -> Result<()> {
        let display = {
            let displays = self.displays.read().await;
            displays.get(session_id).cloned()
        };

        if let Some(display) = display {
            let output = Command::new("xdotool")
                .arg("mouseup")
                .arg(button.to_string())
                .env("DISPLAY", &display)
                .output()
                .await
                .context("Failed to execute xdotool mouseup")?;

            if !output.status.success() {
                error!("xdotool mouseup failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Ok(())
    }

    pub async fn handle_key_down(&self, session_id: &str, key: &str) -> Result<()> {
        let display = {
            let displays = self.displays.read().await;
            displays.get(session_id).cloned()
        };

        if let Some(display) = display {
            let xdotool_key = map_key_to_xdotool(key);
            let output = Command::new("xdotool")
                .arg("keydown")
                .arg(&xdotool_key)
                .env("DISPLAY", &display)
                .output()
                .await
                .context("Failed to execute xdotool keydown")?;

            if !output.status.success() {
                debug!("xdotool keydown {} failed: {}", xdotool_key, String::from_utf8_lossy(&output.stderr));
            }
        }
        Ok(())
    }

    pub async fn handle_key_up(&self, session_id: &str, key: &str) -> Result<()> {
        let display = {
            let displays = self.displays.read().await;
            displays.get(session_id).cloned()
        };

        if let Some(display) = display {
            let xdotool_key = map_key_to_xdotool(key);
            let output = Command::new("xdotool")
                .arg("keyup")
                .arg(&xdotool_key)
                .env("DISPLAY", &display)
                .output()
                .await
                .context("Failed to execute xdotool keyup")?;

            if !output.status.success() {
                debug!("xdotool keyup {} failed: {}", xdotool_key, String::from_utf8_lossy(&output.stderr));
            }
        }
        Ok(())
    }
}

fn map_key_to_xdotool(key: &str) -> String {
    match key {
        "Enter" => "Return".to_string(),
        "Escape" => "Escape".to_string(),
        "Backspace" => "BackSpace".to_string(),
        "Tab" => "Tab".to_string(),
        "Shift" => "Shift_L".to_string(),
        "Control" => "Control_L".to_string(),
        "Alt" => "Alt_L".to_string(),
        "Meta" => "Super_L".to_string(),
        "ArrowUp" => "Up".to_string(),
        "ArrowDown" => "Down".to_string(),
        "ArrowLeft" => "Left".to_string(),
        "ArrowRight" => "Right".to_string(),
        "Delete" => "Delete".to_string(),
        "Home" => "Home".to_string(),
        "End" => "End".to_string(),
        "PageUp" => "Page_Up".to_string(),
        "PageDown" => "Page_Down".to_string(),
        " " => "space".to_string(),
        key if key.len() == 1 => key.to_lowercase(),
        _ => key.to_string(),
    }
}
