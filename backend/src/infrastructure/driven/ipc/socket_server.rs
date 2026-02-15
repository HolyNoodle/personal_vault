use anyhow::{Context, Result};
use shared::{AppMessage, PlatformMessage};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Manages IPC socket server for app communication
pub struct IpcSocketServer {
    socket_path: PathBuf,
    // Removed connections field referencing missing Connection
}


impl IpcSocketServer {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            // Removed connections initialization referencing missing Connection
        }
    }

    /// Start the IPC socket server
    pub async fn start(&self) -> Result<()> {
        // Remove existing socket file if it exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .context("Failed to remove existing socket file")?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .context("Failed to bind Unix socket")?;

        info!("IPC socket server listening on {:?}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        stream: UnixStream,
    ) -> Result<()> {
        info!("New IPC connection established");

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Create channels for bidirectional communication
        let (_tx_to_app, mut rx_from_backend) = mpsc::unbounded_channel::<PlatformMessage>();
        let (_tx_to_backend, _rx_from_app) = mpsc::unbounded_channel::<AppMessage>();

        // Spawn task to send messages to app
        tokio::spawn(async move {
            while let Some(msg) = rx_from_backend.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Err(e) = writer.write_all(format!("{}\n", json).as_bytes()).await {
                        error!("Failed to write to app: {}", e);
                        break;
                    }
                } else {
                    error!("Failed to serialize message to app");
                }
            }
            debug!("App writer task ended");
        });

        // Read messages from app
        let mut line = String::new();
        let session_id: Option<String> = None;

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("App disconnected");
                    break;
                }
                Ok(_) => {
                    match serde_json::from_str::<AppMessage>(&line) {
                        Ok(msg) => {
                            debug!("Received from app: {:?}", msg);

                            // Handle message based on type
                            match &msg {
                                AppMessage::State { path, selected, actions, metadata: _ } => {
                                    info!(
                                        "App state updated: path={}, selected={:?}, actions={:?}",
                                        path, selected, actions
                                    );
                                    // TODO: Update frontend with app state
                                }
                                AppMessage::DownloadData { filename, data: _ } => {
                                    info!("Received download data for: {}", filename);
                                    // TODO: Send file to frontend
                                }
                                AppMessage::Success { operation, message } => {
                                    info!("Operation succeeded: {} - {:?}", operation, message);
                                }
                                AppMessage::Error { message, code } => {
                                    error!("App error: {} (code: {:?})", message, code);
                                }
                                AppMessage::Log { level, message } => {
                                    match level {
                                        shared::LogLevel::Debug => debug!("App: {}", message),
                                        shared::LogLevel::Info => info!("App: {}", message),
                                        shared::LogLevel::Warn => warn!("App: {}", message),
                                        shared::LogLevel::Error => error!("App: {}", message),
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse message from app: {} - Line: {}", e, line);
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from app: {}", e);
                    break;
                }
            }
        }

        // Clean up connection
        if let Some(sid) = session_id {
            // Removed connection cleanup referencing missing Connection
            info!("Removed connection for session: {}", sid);
        }

        Ok(())
    }

    // Removed orphaned and mis-indented code causing unexpected closing delimiter
    // ...existing code...
}

impl Drop for IpcSocketServer {
    fn drop(&mut self) {
        // Clean up socket file
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}
