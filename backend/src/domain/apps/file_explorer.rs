use crate::domain::aggregates::{AppId, SandboxConstraints, ResourceLimits};
use crate::domain::value_objects::UserRole;

/// File Explorer Application
/// First application in the platform - browse files with PDF/image/video preview
pub struct FileExplorerApp {
    pub metadata: AppMetadata,
}

impl FileExplorerApp {
    pub fn new() -> Self {
        Self {
            metadata: AppMetadata {
                app_id: AppId::new("file-explorer-v1"),
                name: "File Explorer".to_string(),
            },
        }
    }

    /// Get default sandbox constraints for file explorer based on user role
    pub fn sandbox_constraints(
        &self,
        allowed_paths: Vec<String>,
        user_role: &UserRole,
        enable_watermarking: bool,
    ) -> SandboxConstraints {
        SandboxConstraints {
            allowed_paths,
            resource_limits: ResourceLimits {
                cpu_percent: 50,
                memory_mb: 512,
                max_pids: 100,
            },
            network_isolated: true,
            watermarking: enable_watermarking,
            record_session: matches!(user_role, UserRole::Client),
        }
    }

    // Removed unused method validate_path
}

impl Default for FileExplorerApp {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AppMetadata {
    pub app_id: AppId,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_explorer_creation() {
        let app = FileExplorerApp::new();
        assert_eq!(app.metadata.app_id.as_str(), "file-explorer-v1");
        assert_eq!(app.metadata.name, "File Explorer");
    }

    #[test]
    fn test_sandbox_constraints() {
        use crate::domain::value_objects::UserRole;
        let app = FileExplorerApp::new();
        
        let owner_constraints = app.sandbox_constraints(
            vec!["/mnt/user_files".to_string()],
            &UserRole::Owner,
            false
        );
        assert_eq!(owner_constraints.resource_limits.cpu_percent, 50);
        assert_eq!(owner_constraints.resource_limits.memory_mb, 512);
        assert!(owner_constraints.network_isolated);
        assert!(!owner_constraints.record_session);
        
        let client_constraints = app.sandbox_constraints(
            vec!["/mnt/user_files/shared".to_string()],
            &UserRole::Client,
            true
        );
        assert!(client_constraints.watermarking);
        assert!(client_constraints.record_session);
    }
}
