use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Command as TokioCommand, ChildStdout};
use tracing::info;

// ...existing code...
// Removed import for deleted VideoStreamingPort

/// FFmpeg encoder for capturing X11 display and encoding to H.264
pub struct FfmpegManager {
    // Removed encoders field referencing missing FfmpegEncoder
}


impl FfmpegManager {
    pub fn new() -> Self {
        Self {}
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

        // Removed creation and insertion of FfmpegEncoder

        Ok(stdout)
    }

    async fn stop_encoder(&self, session_id: &str) -> Result<()> {
        // Removed encoder cleanup referencing FfmpegEncoder
        info!("Stopping FFmpeg encoder for session {}", session_id);
        // TODO: Implement process tracking and cleanup if needed
        Ok(())
    }

    // Removed orphaned code causing unexpected closing delimiter
}

// Removed trait implementation for deleted VideoStreamingPort
