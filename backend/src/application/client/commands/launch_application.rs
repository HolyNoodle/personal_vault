use anyhow::Result;
use crate::domain::aggregates::{ApplicationSession, AppId, AppVideoConfig};
use crate::domain::value_objects::UserRole;
use crate::domain::apps::FileExplorerApp;
use crate::application::ports::{
    ApplicationSessionRepository, ApplicationLauncherPort, SandboxIsolationPort,
    // Removed unused imports: AppVideoStreamingPort, BrowserLaunchInfo
};
use std::sync::Arc;

/// Command to launch an application session
pub struct LaunchApplicationCommand {
    pub app_id: String,
    pub user_id: String,
    pub user_role: UserRole,
    pub allowed_paths: Vec<String>,
    pub video_width: u16,
    pub video_height: u16,
    pub video_framerate: u8,
    pub enable_watermarking: bool,
    pub timeout_minutes: u32,
}

/// Response from launching application
pub struct LaunchApplicationResponse {
    pub session_id: String,
    pub webrtc_offer: String,
}

/// Application launcher service
pub struct ApplicationLauncherService {
    session_repository: Arc<dyn ApplicationSessionRepository>,
    launcher: Arc<dyn ApplicationLauncherPort>,
    sandbox_isolation: Arc<dyn SandboxIsolationPort>,
    // Video session handler for actual streaming
    create_session_handler: Arc<crate::application::client::commands::CreateSessionHandler>,
    webrtc_adapter: Arc<crate::infrastructure::driving::WebRTCAdapter>,
}

impl ApplicationLauncherService {
    pub fn new(
        session_repository: Arc<dyn ApplicationSessionRepository>,
        launcher: Arc<dyn ApplicationLauncherPort>,
        sandbox_isolation: Arc<dyn SandboxIsolationPort>,
        create_session_handler: Arc<crate::application::client::commands::CreateSessionHandler>,
        webrtc_adapter: Arc<crate::infrastructure::driving::WebRTCAdapter>,
    ) -> Self {
        Self {
            session_repository,
            launcher,
            sandbox_isolation,
            create_session_handler,
            webrtc_adapter,
        }
    }

    pub async fn execute(&self, command: LaunchApplicationCommand) -> Result<LaunchApplicationResponse> {
        // For now, we only support file explorer
        let app_id = AppId::new(&command.app_id);
        if app_id.as_str() != "file-explorer-v1" {
            return Err(anyhow::anyhow!("Unsupported application: {}", command.app_id));
        }

        let app = FileExplorerApp::new();

        // Create video config
        let video_config = AppVideoConfig {
            width: command.video_width,
            height: command.video_height,
            framerate: command.video_framerate,
            codec: crate::domain::aggregates::AppVideoCodec::H264,
        };

        // Get sandbox constraints based on user role
        let constraints = app.sandbox_constraints(
            command.allowed_paths,
            &command.user_role,
            command.enable_watermarking,
        );

        // Create sandboxed execution
        let execution = crate::domain::aggregates::SandboxedExecution {
            sandbox_id: None,
            video_config: video_config.clone(),
            constraints,
            user_role: command.user_role,
        };

        // Create session
        let mut session = ApplicationSession::new(
            app.metadata.app_id.clone(),
            command.user_id,
            execution,
            command.timeout_minutes,
        );

        // Save session
        self.session_repository.save(&session).await?;

        // Launch sandboxed application
        let app_config = crate::application::ports::ApplicationConfig {
            app_id: app.metadata.app_id.clone(),
            name: app.metadata.name.clone(),
            sandboxed_binary: Some(app.binary_path().to_string()),
            browser_bundle: None,
        };

        let sandbox_id = self.launcher.launch_sandboxed(&session, &app_config).await?;

        // Update session with sandbox ID
        session.execution.sandbox_id = Some(sandbox_id);
        session.mark_ready();
        self.session_repository.save(&session).await?;

        // Create video session for streaming the application
        let video_command = crate::application::client::commands::CreateSessionCommand {
            user_id: session.user_id.clone(),
            config: crate::domain::aggregates::VideoConfig {
                width: command.video_width,
                height: command.video_height,
                framerate: command.video_framerate,
                codec: crate::domain::aggregates::VideoCodec::H264,
            },
            application: "file-explorer".to_string(), // Custom Rust file explorer app
        };
        
        let video_result = self.create_session_handler.handle(
            video_command,
            Arc::clone(&self.webrtc_adapter),
        ).await?;

        // Return session info
        Ok(LaunchApplicationResponse {
            session_id: video_result.session_id,
            webrtc_offer: video_result.websocket_url,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_command_creation() {
        let cmd = LaunchApplicationCommand {
            app_id: "file-explorer-v1".to_string(),
            user_id: "user123".to_string(),
            execution_mode: ExecutionModeRequest::Sandboxed {
                video_width: 1920,
                video_height: 1080,
                video_framerate: 30,
                enable_watermarking: false,
            },
            allowed_paths: vec!["/mnt/user_files".to_string()],
            timeout_minutes: 120,
        };

        assert_eq!(cmd.app_id, "file-explorer-v1");
        assert_eq!(cmd.timeout_minutes, 120);
    }
}
