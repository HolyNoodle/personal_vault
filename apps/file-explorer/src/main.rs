use futures_util::future::FutureExt;
use anyhow::{Context, Result};
use base64::Engine;
use eframe::egui;
use shared::{AppMessage, PlatformMessage};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{error, info};

// Get socket path from environment or use default
fn get_socket_path() -> String {
    std::env::var("IPC_SOCKET_PATH")
        .unwrap_or_else(|_| "/tmp/ipc/sandbox-ipc.sock".to_string())
}

#[derive(Default)]
struct FileItem {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

struct FileExplorerApp {
        search_query: String,
    current_path: PathBuf,
    items: Vec<FileItem>,
    selected_index: Option<usize>,
    error_message: Option<String>,
    tx: mpsc::UnboundedSender<AppMessage>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<PlatformMessage>>>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl FileExplorerApp {
    fn new(
        tx: mpsc::UnboundedSender<AppMessage>,
        rx: Arc<Mutex<mpsc::UnboundedReceiver<PlatformMessage>>>,
        shutdown: Arc<tokio::sync::Notify>,
    ) -> Self {
        let mut app = Self {
            current_path: PathBuf::from("/data/storage"),
            items: Vec::new(),
            selected_index: None,
            error_message: None,
            tx,
            rx,
            search_query: String::new(),
            shutdown,
        };
        app.refresh_directory();
        app
    }

    fn refresh_directory(&mut self) {
        self.items.clear();
        self.selected_index = None;

        match std::fs::read_dir(&self.current_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        self.items.push(FileItem {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: entry.path(),
                            is_dir: metadata.is_dir(),
                            size: metadata.len(),
                        });
                    }
                }
                self.items.sort_by(|a, b| {
                    match (a.is_dir, b.is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to read directory: {}", e));
            }
        }

        // Send state to platform
        let selected_path = self
            .selected_index
            .and_then(|idx| self.items.get(idx))
            .map(|item| item.path.to_string_lossy().to_string());

        let actions = self.get_contextual_actions();

        let _ = self.tx.send(AppMessage::State {
            path: self.current_path.display().to_string(),
            selected: selected_path,
            actions,
            metadata: serde_json::json!({}),
        });
    }

    fn get_contextual_actions(&self) -> Vec<String> {
        let mut actions = Vec::new();

        // Always show upload when in a directory
        actions.push("upload".to_string());

        if let Some(idx) = self.selected_index {
            if let Some(item) = self.items.get(idx) {
                if !item.is_dir {
                    actions.push("download".to_string());
                }
                actions.push("delete".to_string());
            }
        }

        actions
    }

    fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path;
        self.refresh_directory();
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf());
        }
    }

    fn handle_platform_message(&mut self, msg: PlatformMessage) {
        match msg {
            PlatformMessage::UploadFile { filename, data } => {
                let target_path = self.current_path.join(&filename);
                match base64::engine::general_purpose::STANDARD.decode(data) {
                    Ok(bytes) => match std::fs::write(&target_path, bytes) {
                        Ok(_) => {
                            info!("File uploaded: {}", filename);
                            self.refresh_directory();
                            let _ = self.tx.send(AppMessage::Success {
                                operation: "upload".to_string(),
                                message: Some(format!("File {} uploaded successfully", filename)),
                            });
                        }
                        Err(e) => {
                            error!("Failed to write file: {}", e);
                            let _ = self.tx.send(AppMessage::Error {
                                message: format!("Failed to write file: {}", e),
                                code: None,
                            });
                        }
                    },
                    Err(e) => {
                        error!("Failed to decode file data: {}", e);
                        let _ = self.tx.send(AppMessage::Error {
                            message: format!("Failed to decode file: {}", e),
                            code: None,
                        });
                    }
                }
            }
            PlatformMessage::RequestDownload => {
                if let Some(idx) = self.selected_index {
                    if let Some(item) = self.items.get(idx) {
                        if !item.is_dir {
                            match std::fs::read(&item.path) {
                                Ok(bytes) => {
                                    let _ = self.tx.send(AppMessage::DownloadData {
                                        filename: item.name.clone(),
                                        data: bytes,
                                    });
                                }
                                Err(e) => {
                                    error!("Failed to read file: {}", e);
                                    let _ = self.tx.send(AppMessage::Error {
                                        message: format!("Failed to read file: {}", e),
                                        code: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            PlatformMessage::Delete => {
                if let Some(idx) = self.selected_index {
                    if let Some(item) = self.items.get(idx) {
                        let item_name = item.name.clone();
                        let item_path = item.path.clone();
                        let is_dir = item.is_dir;
                        
                        let result = if is_dir {
                            std::fs::remove_dir_all(&item_path)
                        } else {
                            std::fs::remove_file(&item_path)
                        };

                        match result {
                            Ok(_) => {
                                info!("Deleted: {}", item_path.display());
                                self.refresh_directory();
                                let _ = self.tx.send(AppMessage::Success {
                                    operation: "delete".to_string(),
                                    message: Some(format!("Deleted {}", item_name)),
                                });
                            }
                            Err(e) => {
                                error!("Failed to delete: {}", e);
                                let _ = self.tx.send(AppMessage::Error {
                                    message: format!("Failed to delete: {}", e),
                                    code: None,
                                });
                            }
                        }
                    }
                }
            }
            PlatformMessage::Command { command, params } => {
                info!("Received command: {} with params: {:?}", command, params);
            }
        }
    }
}

impl eframe::App for FileExplorerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check shutdown signal and close window if triggered
        if self.shutdown.notified().now_or_never().is_some() {
            std::process::exit(0);
        }
        // Process incoming messages from platform
        let mut messages = Vec::new();
        if let Ok(mut rx) = self.rx.try_lock() {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }
        for msg in messages {
            self.handle_platform_message(msg);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Split view: left = file/folder list, right = file preview
            ui.columns(2, |columns| {
                // LEFT PANEL: File/folder list and navigation
                columns[0].vertical(|ui| {
                    ui.horizontal(|ui| {
                        // Only show up button if not at root
                        let root = PathBuf::from("/data/storage");
                        if self.current_path != root {
                            if ui.button("⬆ Up").clicked() {
                                self.go_up();
                            }
                        }
                        // Show path as / if at root, else relative
                        let path_display = if self.current_path == root {
                            "/".to_string()
                        } else {
                            self.current_path.strip_prefix(&root)
                                .map(|p| format!("/{}", p.display()))
                                .unwrap_or_else(|_| self.current_path.display().to_string())
                        };
                        ui.label(format!("📁 {}", path_display));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("🔍 Search:");
                        ui.text_edit_singleline(&mut self.search_query);
                    });
                    ui.separator();
                    if let Some(ref error) = self.error_message {
                        ui.colored_label(egui::Color32::RED, error);
                        ui.separator();
                    }
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .enable_scrolling(true)
                        .show(ui, |ui| {
                        let mut navigate_to = None;
                        for (idx, item) in self.items.iter().enumerate() {
                            if !self.search_query.is_empty() && !item.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
                                continue;
                            }
                            let is_selected = self.selected_index == Some(idx);
                            let icon = if item.is_dir { "📁" } else { "📄" };
                            let response = ui.selectable_label(
                                is_selected,
                                format!(
                                    "{} {} {}",
                                    icon,
                                    item.name,
                                    if item.is_dir {
                                        String::new()
                                    } else {
                                        format!("({} bytes)", item.size)
                                    }
                                ),
                            );
                            if response.clicked() {
                                if item.is_dir {
                                    self.selected_index = Some(idx);
                                    navigate_to = Some(item.path.clone());
                                } else {
                                    self.selected_index = Some(idx);
                                }
                            }
                        }
                        if let Some(path) = navigate_to {
                            self.navigate_to(path);
                        }
                    });
                });
                // RIGHT PANEL: File preview
                columns[1].vertical(|ui| {
                    ui.heading("Preview");
                    ui.separator();
                    if let Some(idx) = self.selected_index {
                        let item = &self.items[idx];
                        if !item.is_dir {
                            let ext = item.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                            match ext.as_str() {
                                "png" | "jpg" | "jpeg" | "bmp" | "gif" => {
                                    match image::open(&item.path) {
                                        Ok(img) => {
                                            let img = img.to_rgba8();
                                            let size = [img.width() as usize, img.height() as usize];
                                            let pixels = img.into_raw();
                                            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                            let texture = ui.ctx().load_texture("preview", color_image, egui::TextureOptions::default());
                                            ui.image(&texture);
                                        }
                                        Err(e) => {
                                            ui.colored_label(egui::Color32::RED, format!("Failed to load image: {}", e));
                                        }
                                    }
                                }
                                "mp4" | "webm" | "mkv" | "avi" => {
                                    ui.label("Video preview not supported in native egui. Download to view.");
                                }
                                "pdf" => {
                                    ui.label("PDF preview not supported in native egui.");
                                    if ui.button("Open PDF").clicked() {
                                        let path = item.path.clone();
                                        // Launch PDF with system viewer
                                        let _ = std::process::Command::new("xdg-open")
                                            .arg(&path)
                                            .spawn();
                                    }
                                }
                                _ => {
                                    match std::fs::read_to_string(&item.path) {
                                        Ok(text) => {
                                            egui::ScrollArea::vertical().show(ui, |ui| {
                                                ui.code(text);
                                            });
                                        }
                                        Err(e) => {
                                            ui.colored_label(egui::Color32::RED, format!("Failed to preview file: {}", e));
                                        }
                                    }
                                }
                            }
                        } else {
                            ui.label("Select a file to preview its contents.");
                        }
                    } else {
                        ui.label("Select a file to preview its contents.");
                    }
                });
            });
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Log parent PID and command-line arguments for diagnosis
    let pid = std::process::id();
    let ppid = unsafe { libc::getppid() };
    let args: Vec<String> = std::env::args().collect();
    info!("file-explorer startup: pid={}, ppid={}, args={:?}", pid, ppid, args);

    info!("Starting file explorer app...");

    let socket_path = get_socket_path();
    info!("Attempting to connect to IPC socket at {}", socket_path);

    // Connect to platform via Unix socket
    let socket = UnixStream::connect(&socket_path)
        .await
        .context(format!("Failed to connect to platform socket at {}", socket_path))?;

    info!("Connected to platform at {}", socket_path);

    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    // Create channels for bidirectional communication
    let (tx_to_platform, mut rx_from_app) = mpsc::unbounded_channel::<AppMessage>();
    let (tx_to_app, rx_from_platform) = mpsc::unbounded_channel::<PlatformMessage>();
    let rx_from_platform = Arc::new(Mutex::new(rx_from_platform));

    // Spawn task to read from socket and exit app if disconnected
    let tx_to_app_clone = tx_to_app.clone();
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("Platform disconnected, shutting down file-explorer.");
                    shutdown_clone.notify_waiters();
                    break;
                }
                Ok(_) => {
                    if let Ok(msg) = serde_json::from_str::<PlatformMessage>(&line) {
                        let _ = tx_to_app_clone.send(msg);
                    } else {
                        error!("Failed to parse message from platform: {}", line);
                    }
                }
                Err(e) => {
                    error!("Error reading from socket: {}", e);
                    shutdown_clone.notify_waiters();
                    break;
                }
            }
        }
    });

    // Spawn task to write to socket
    tokio::spawn(async move {
        while let Some(msg) = rx_from_app.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if let Err(e) = writer.write_all(format!("{}\n", json).as_bytes()).await {
                    error!("Failed to write to socket: {}", e);
                    break;
                }
            }
        }
    });

    // Run the GUI in the main thread
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
            .with_title("File Explorer")
            .with_decorations(false)
            .with_fullscreen(true),
        ..Default::default()
    };

    let tx_clone = tx_to_platform.clone();
    let rx_clone = rx_from_platform.clone();
    let shutdown_gui = shutdown.clone();
    eframe::run_native(
        "File Explorer",
        options,
        Box::new(move |_cc| Ok(Box::new(FileExplorerApp::new(tx_clone, rx_clone, shutdown_gui))))
    );
    // After GUI closes, check if shutdown was triggered
    // Check if shutdown was triggered (non-blocking)
    let shutdown_triggered = shutdown.notified().now_or_never().is_some();
    if shutdown_triggered {
        info!("File-explorer exiting due to platform disconnect.");
        std::process::exit(0);
    }
    Ok(())
}
