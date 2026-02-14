use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::{Command as TokioCommand, ChildStdout};
use std::collections::HashMap;
use tracing::{error, info};

use crate::domain::aggregates::VideoSessionId;
use crate::application::ports::VideoStreamingPort;

/// FFmpeg encoder for capturing X11 display and encoding to H.264
pub struct FfmpegManager {
    encoders: Arc<RwLock<HashMap<String, FfmpegEncoder>>>,
}

struct FfmpegEncoder {
    process: Option<tokio::process::Child>,
    stdout: Option<ChildStdout>,
}

impl FfmpegManager {
    pub fn new() -> Self {
        Self {
            encoders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn start_encoder(
        &self,
        session_id: &str,
        display_str: &str,
        width: u16,
        height: u16,
        framerate: u8,
    ) -> Result<ChildStdout> {
        info!(
            "Starting FFmpeg encoder for session {} on display {} ({}x{}@{}fps)",
            session_id, display_str, width, height, framerate
        );

        let resolution = format!("{}x{}", width, height);

        // Output H.264 to stdout for WebRTC streaming
        let mut child = TokioCommand::new("ffmpeg")
            .arg("-f")
            .arg("x11grab")
            .arg("-video_size")
            .arg(&resolution)
            .arg("-framerate")
            .arg(framerate.to_string())
            .arg("-i")
            .arg(display_str)
            .arg("-c:v")
            .arg("libvpx")
            .arg("-b:v")
            .arg("1M")
            .arg("-deadline")
            .arg("realtime")
            .arg("-cpu-used")
            .arg("8")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-g")
            .arg("60")
            .arg("-f")
            .arg("ivf")
            .arg("pipe:1")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start FFmpeg. Make sure FFmpeg is installed.")?;

        let stdout = child.stdout.take()
            .context("Failed to capture FFmpeg stdout")?;

        info!("FFmpeg encoder started for session {}, streaming to WebRTC", session_id);

        let encoder = FfmpegEncoder {
            process: Some(child),
            stdout: None,  // We return the stdout, don't store it
        };

        let mut encoders = self.encoders.write().await;
        encoders.insert(session_id.to_string(), encoder);

        Ok(stdout)
    }

    async fn stop_encoder(&self, session_id: &str) -> Result<()> {
        let mut encoders = self.encoders.write().await;
        if let Some(mut encoder) = encoders.remove(session_id) {
            info!("Stopping FFmpeg encoder for session {}", session_id);
            if let Some(mut child) = encoder.process.take() {
                if let Err(e) = child.kill().await {
                    error!("Failed to kill FFmpeg process: {}", e);
                }
                let _ = child.wait().await;
            }
        }
        Ok(())
    }

    async fn check_running(&self, session_id: &str) -> Result<bool> {
        let mut encoders = self.encoders.write().await;
        if let Some(encoder) = encoders.get_mut(session_id) {
            if let Some(child) = &mut encoder.process {
                Ok(child.try_wait().ok().flatten().is_none())
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

impl VideoStreamingPort for FfmpegManager {
    async fn start_session(&self, session_id: &VideoSessionId, display: &str, width: u16, height: u16, framerate: u8) -> Result<ChildStdout> {
        self.start_encoder(session_id.as_str(), display, width, height, framerate).await
    }

    async fn stop_session(&self, session_id: &VideoSessionId) -> Result<()> {
        self.stop_encoder(session_id.as_str()).await
    }

    async fn is_running(&self, session_id: &VideoSessionId) -> Result<bool> {
        self.check_running(session_id.as_str()).await
    }
}
