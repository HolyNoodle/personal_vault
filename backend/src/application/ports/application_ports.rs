use anyhow::Result;
use async_trait::async_trait;
use crate::domain::aggregates::{
    ApplicationSession, SessionId, AppId, SandboxConstraints,
};

/// Port for application session repository
#[async_trait]
pub trait ApplicationSessionRepository: Send + Sync {
    async fn save(&self, session: &ApplicationSession) -> Result<()>;
    async fn find_by_id(&self, id: &SessionId) -> Result<Option<ApplicationSession>>;
    async fn find_by_user_id(&self, user_id: &str) -> Result<Vec<ApplicationSession>>;
    async fn find_active_sessions(&self) -> Result<Vec<ApplicationSession>>;
    async fn delete(&self, id: &SessionId) -> Result<()>;
    async fn update_activity(&self, id: &SessionId) -> Result<()>;
}

/// Port for application launcher (creates and manages application instances)
#[async_trait]
pub trait ApplicationLauncherPort: Send + Sync {
    /// Launch application in sandboxed mode (returns sandbox ID)
    async fn launch_sandboxed(
        &self,
        session: &ApplicationSession,
        app_config: &ApplicationConfig,
    ) -> Result<String>;
    
    /// Prepare browser bundle for client-side execution (returns bundle URL + JWT)
    async fn prepare_browser(
        &self,
        session: &ApplicationSession,
        app_config: &ApplicationConfig,
    ) -> Result<BrowserLaunchInfo>;
    
    /// Terminate application instance
    async fn terminate(&self, session_id: &SessionId) -> Result<()>;
    
    /// Check if application is running
    async fn is_running(&self, session_id: &SessionId) -> Result<bool>;
}

/// Port for sandbox isolation (Linux namespaces, Landlock, cgroups, etc.)
#[async_trait]
pub trait SandboxIsolationPort: Send + Sync {
    /// Create isolated sandbox environment
    async fn create_sandbox(
        &self,
        session_id: &SessionId,
        constraints: &SandboxConstraints,
    ) -> Result<SandboxHandle>;
    
    /// Execute command inside sandbox
    async fn execute_in_sandbox(
        &self,
        sandbox_handle: &SandboxHandle,
        command: &str,
        args: &[&str],
    ) -> Result<()>;
    
    /// Mount file system path into sandbox (read-only via Landlock)
    async fn mount_path(
        &self,
        sandbox_handle: &SandboxHandle,
        host_path: &str,
        sandbox_path: &str,
        readonly: bool,
    ) -> Result<()>;
    
    /// Destroy sandbox and cleanup resources
    async fn destroy_sandbox(&self, sandbox_handle: &SandboxHandle) -> Result<()>;
    
    /// Get resource usage stats
    async fn get_resource_usage(&self, sandbox_handle: &SandboxHandle) -> Result<ResourceUsage>;
}

/// Port for video streaming (FFmpeg + WebRTC for sandboxed mode)
#[async_trait]
pub trait VideoStreamingPort: Send + Sync {
    /// Start video capture from virtual display
    async fn start_capture(
        &self,
        session_id: &SessionId,
        display: &str,
        video_config: &crate::domain::aggregates::AppVideoConfig,
    ) -> Result<StreamHandle>;
    
    /// Create WebRTC peer connection
    async fn create_peer_connection(
        &self,
        session_id: &SessionId,
        stream_handle: &StreamHandle,
    ) -> Result<WebRTCOffer>;
    
    /// Handle WebRTC answer from client
    async fn handle_answer(
        &self,
        session_id: &SessionId,
        answer: WebRTCAnswer,
    ) -> Result<()>;
    
    /// Stop video stream
    async fn stop_stream(&self, session_id: &SessionId) -> Result<()>;
}

/// Port for input forwarding (keyboard/mouse events to sandbox)
#[async_trait]
pub trait InputForwardingPort: Send + Sync {
    /// Forward input event to sandboxed application
    async fn forward_input(
        &self,
        session_id: &SessionId,
        event: InputEvent,
    ) -> Result<()>;
}

/// Port for file system operations (for browser mode API)
#[async_trait]
pub trait FileSystemPort: Send + Sync {
    /// List files in directory
    async fn list_directory(&self, user_id: &str, path: &str) -> Result<Vec<FileEntry>>;
    
    /// Read file metadata
    async fn get_metadata(&self, user_id: &str, path: &str) -> Result<FileMetadata>;
    
    /// Read file content (for preview/download)
    async fn read_file(&self, user_id: &str, path: &str) -> Result<Vec<u8>>;
    
    /// Write file (upload)
    async fn write_file(&self, user_id: &str, path: &str, content: &[u8]) -> Result<()>;
    
    /// Delete file or directory
    async fn delete(&self, user_id: &str, path: &str) -> Result<()>;
    
    /// Move/rename file
    async fn move_file(&self, user_id: &str, from: &str, to: &str) -> Result<()>;
    
    /// Create directory
    async fn create_directory(&self, user_id: &str, path: &str) -> Result<()>;
    
    /// Get storage quota info
    async fn get_quota(&self, user_id: &str) -> Result<QuotaInfo>;
}

// Supporting types

#[derive(Debug, Clone)]
pub struct ApplicationConfig {
    pub app_id: AppId,
    pub name: String,
    pub sandboxed_binary: Option<String>,
    pub browser_bundle: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BrowserLaunchInfo {
    pub bundle_url: String,
    pub jwt_token: String,
    pub api_endpoint: String,
}

#[derive(Debug, Clone)]
pub struct SandboxHandle {
    pub sandbox_id: String,
    pub pid_namespace: String,
    pub mount_namespace: String,
    pub network_namespace: String,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub pid_count: u16,
}

#[derive(Debug, Clone)]
pub struct StreamHandle {
    pub stream_id: String,
    pub ffmpeg_pid: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebRTCOffer {
    pub sdp: String,
    pub ice_candidates: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebRTCAnswer {
    pub sdp: String,
    pub ice_candidates: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    MouseWheel { delta_x: i32, delta_y: i32 },
    KeyDown { key: String },
    KeyUp { key: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub created: chrono::DateTime<chrono::Utc>,
    pub mime_type: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuotaInfo {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub file_count: u32,
}
