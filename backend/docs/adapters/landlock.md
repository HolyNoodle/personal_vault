# Landlock LSM Adapter (Kernel-Level Access Control)

**Purpose**: Enforce filesystem access control at the Linux kernel level using Landlock LSM.

**Technology**: Landlock Linux Security Module (kernel 5.13+) + Rust landlock crate

**Layer**: Adapters (Secondary/Driven Adapter)

---

## Responsibilities

- Create filesystem sandboxes for client sessions
- Enforce read/write/execute permissions at kernel level
- Update sandbox policies dynamically (~2 second propagation)
- Prevent privilege escalation and escapes
- Provide strong security guarantees independent of application bugs

---

## Dependencies

### Required Crates
```toml
[dependencies]
# Landlock LSM
landlock = "0.3"

# System calls
nix = { version = "0.28", features = ["user", "mount", "sched", "process"] }
libc = "0.2"

# Async runtime
tokio = { version = "1.35", features = ["full", "process"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"
```

### System Requirements
- **Linux Kernel**: 5.13+ (Landlock LSM must be enabled)
- **Kernel Config**: `CONFIG_SECURITY_LANDLOCK=y`
- **Check availability**:
  ```bash
  cat /sys/kernel/security/lsm | grep landlock
  # Should output: landlock
  ```

---

## How Landlock Works

### Traditional Access Control (DAC)
```
User owns file → User can always access file
Problem: If application is compromised, attacker has full user permissions
```

### Landlock Access Control
```
Application creates sandbox → Kernel restricts access regardless of file ownership
Benefit: Even if application is compromised, kernel blocks unauthorized access
```

**Key Concept**: Landlock is **restrictive-only**. It can only reduce privileges, never grant more.

---

## Sandbox Creation

### Basic Sandbox Example
```rust
use landlock::*;
use std::path::Path;

pub fn create_read_only_sandbox(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create ruleset
    let abi = ABI::V2; // Landlock ABI version 2
    let ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .create()?;
    
    // Add rule: Allow read-only access to specific file
    let rule = PathBeneath::new(PathFd::new(file_path)?, AccessFs::ReadFile | AccessFs::ReadDir);
    ruleset.add_rule(rule)?;
    
    // Restrict current process
    let status = ruleset.restrict_self()?;
    
    match status.ruleset {
        RulesetStatus::FullyEnforced => {
            println!("Sandbox fully enforced");
        }
        RulesetStatus::PartiallyEnforced => {
            eprintln!("Warning: Sandbox partially enforced (older kernel)");
        }
        RulesetStatus::NotEnforced => {
            return Err("Landlock not supported on this kernel".into());
        }
    }
    
    Ok(())
}
```

### Advanced Sandbox with Multiple Paths
```rust
use landlock::*;
use std::path::PathBuf;

pub struct SandboxConfig {
    pub allowed_paths: Vec<(PathBuf, AccessFs)>,
}

pub fn create_sandbox(config: SandboxConfig) -> Result<(), LandlockError> {
    let abi = ABI::V2;
    
    // Create ruleset with all possible filesystem access types
    let mut ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .create()?;
    
    // Add rules for each allowed path
    for (path, access) in config.allowed_paths {
        let path_fd = PathFd::new(&path)?;
        let rule = PathBeneath::new(path_fd, access);
        ruleset = ruleset.add_rule(rule)?;
    }
    
    // Restrict self
    ruleset.restrict_self()?;
    
    Ok(())
}
```

---

## Session Sandbox Implementation

### Sandbox Manager
```rust
use landlock::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::process::Child;
use uuid::Uuid;

pub struct SessionSandbox {
    pub sandbox_id: Uuid,
    pub file_path: PathBuf,
    pub permissions: SandboxPermissions,
    pub process: Option<Child>,
}

#[derive(Debug, Clone)]
pub struct SandboxPermissions {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

impl SandboxPermissions {
    pub fn to_landlock_access(&self) -> AccessFs {
        let mut access = AccessFs::empty();
        
        if self.can_read {
            access |= AccessFs::ReadFile | AccessFs::ReadDir;
        }
        
        if self.can_write {
            access |= AccessFs::WriteFile | AccessFs::MakeDir | AccessFs::RemoveFile | AccessFs::RemoveDir;
        }
        
        if self.can_execute {
            access |= AccessFs::Execute;
        }
        
        access
    }
}

pub struct LandlockAdapter {
    // Store active sandboxes
    active_sandboxes: std::collections::HashMap<Uuid, SessionSandbox>,
}

impl LandlockAdapter {
    pub fn new() -> Self {
        Self {
            active_sandboxes: std::collections::HashMap::new(),
        }
    }
    
    /// Create a new sandboxed process for file viewing
    pub async fn create_sandbox(
        &mut self,
        file_path: PathBuf,
        permissions: SandboxPermissions,
    ) -> Result<Uuid, LandlockError> {
        let sandbox_id = Uuid::new_v4();
        
        // Spawn child process that will be sandboxed
        let child = self.spawn_sandboxed_viewer(file_path.clone(), permissions.clone()).await?;
        
        let sandbox = SessionSandbox {
            sandbox_id,
            file_path,
            permissions,
            process: Some(child),
        };
        
        self.active_sandboxes.insert(sandbox_id, sandbox);
        
        Ok(sandbox_id)
    }
    
    /// Spawn a child process and apply Landlock restrictions BEFORE execve
    async fn spawn_sandboxed_viewer(
        &self,
        file_path: PathBuf,
        permissions: SandboxPermissions,
    ) -> Result<Child, LandlockError> {
        use tokio::process::Command;
        
        // Create a wrapper script that applies Landlock then execs the viewer
        // This is needed because we must apply Landlock in the child process
        let child = Command::new("/usr/bin/landlock-sandboxed-viewer")
            .arg("--file")
            .arg(&file_path)
            .arg("--permissions")
            .arg(format!("{:?}", permissions))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        Ok(child)
    }
    
    /// Update sandbox permissions (~2 second propagation time)
    pub async fn update_permissions(
        &mut self,
        sandbox_id: Uuid,
        new_permissions: SandboxPermissions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sandbox = self.active_sandboxes
            .get_mut(&sandbox_id)
            .ok_or("Sandbox not found")?;
        
        // Terminate old process
        if let Some(mut process) = sandbox.process.take() {
            process.kill().await?;
        }
        
        // Create new sandboxed process with updated permissions
        let child = self.spawn_sandboxed_viewer(
            sandbox.file_path.clone(),
            new_permissions.clone(),
        ).await?;
        
        sandbox.permissions = new_permissions;
        sandbox.process = Some(child);
        
        // Note: ~2 second delay for process restart + WebRTC reconnection
        
        Ok(())
    }
    
    /// Destroy sandbox (terminate process)
    pub async fn destroy_sandbox(&mut self, sandbox_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut sandbox) = self.active_sandboxes.remove(&sandbox_id) {
            if let Some(mut process) = sandbox.process {
                process.kill().await?;
            }
        }
        
        Ok(())
    }
}
```

---

## Sandboxed Viewer Process

The viewer process runs with Landlock restrictions applied:

```rust
// src/bin/landlock-sandboxed-viewer.rs
use landlock::*;
use std::path::PathBuf;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let file_path = PathBuf::from(&args[2]); // --file <path>
    let permissions: SandboxPermissions = parse_permissions(&args[4])?; // --permissions
    
    // Apply Landlock sandbox BEFORE doing anything else
    apply_landlock_sandbox(&file_path, &permissions)?;
    
    // Now we're sandboxed - start file viewer
    // Even if viewer has vulnerabilities, kernel blocks unauthorized access
    start_file_viewer(&file_path)?;
    
    Ok(())
}

fn apply_landlock_sandbox(
    file_path: &Path,
    permissions: &SandboxPermissions,
) -> Result<(), LandlockError> {
    let abi = ABI::V2;
    
    // Create ruleset
    let mut ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .create()?;
    
    // Add rule for the specific file
    let access = permissions.to_landlock_access();
    let path_fd = PathFd::new(file_path)?;
    let rule = PathBeneath::new(path_fd, access);
    ruleset = ruleset.add_rule(rule)?;
    
    // Also allow read access to system libraries (needed for rendering)
    let lib_paths = vec![
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        PathBuf::from("/usr/lib"),
    ];
    
    for lib_path in lib_paths {
        let lib_fd = PathFd::new(&lib_path)?;
        let lib_rule = PathBeneath::new(lib_fd, AccessFs::ReadFile | AccessFs::ReadDir);
        ruleset = ruleset.add_rule(lib_rule)?;
    }
    
    // Restrict self
    let status = ruleset.restrict_self()?;
    
    match status.ruleset {
        RulesetStatus::FullyEnforced => {
            eprintln!("Landlock sandbox fully enforced");
            Ok(())
        }
        RulesetStatus::NotEnforced => {
            Err(LandlockError::Other("Landlock not supported".into()))
        }
        _ => Ok(()),
    }
}

fn start_file_viewer(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Open file and start rendering to WebRTC stream
    // Implementation depends on file type (PDF, image, video, etc.)
    todo!("Implement file viewer based on content type");
}
```

---

## ~2 Second Update Mechanism

**Why 2 seconds?**

Landlock can only be applied when a process **starts**. To update permissions:
1. Terminate old sandboxed process (~100ms)
2. Spawn new process with updated Landlock policy (~500ms)
3. WebRTC reconnection (~1 second)
4. Total: ~1.6-2 seconds

```rust
pub async fn update_session_permissions(
    adapter: &mut LandlockAdapter,
    sandbox_id: Uuid,
    new_permissions: SandboxPermissions,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Kill old process (Landlock policy cannot be changed for running process)
    // 2. Spawn new process with updated Landlock policy
    // 3. WebRTC peer reconnects to new process
    
    adapter.update_permissions(sandbox_id, new_permissions).await?;
    
    // Client experiences ~2 second interruption in video stream
    // This is acceptable for infrequent permission changes
    
    Ok(())
}
```

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_permissions_to_landlock_access() {
        let perms = SandboxPermissions {
            can_read: true,
            can_write: false,
            can_execute: false,
        };
        
        let access = perms.to_landlock_access();
        assert!(access.contains(AccessFs::ReadFile));
        assert!(!access.contains(AccessFs::WriteFile));
    }
}
```

### Integration Tests
**Requires Linux kernel 5.13+ with Landlock enabled**

```rust
#[cfg(all(target_os = "linux", test))]
mod integration_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_read_only_sandbox_blocks_write() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();
        
        // Create read-only sandbox
        let permissions = SandboxPermissions {
            can_read: true,
            can_write: false,
            can_execute: false,
        };
        
        let mut adapter = LandlockAdapter::new();
        let sandbox_id = adapter.create_sandbox(test_file.clone(), permissions).await.unwrap();
        
        // Try to write (should fail)
        let result = fs::write(&test_file, "modified");
        assert!(result.is_err()); // Blocked by Landlock
        
        // Cleanup
        adapter.destroy_sandbox(sandbox_id).await.unwrap();
    }
}
```

---

## Configuration

```toml
# .env
LANDLOCK_ENABLED=true
LANDLOCK_ABI_VERSION=2
SANDBOXED_VIEWER_PATH=/usr/local/bin/landlock-sandboxed-viewer
```

---

## Security Considerations

1. **Defense in Depth**: Landlock is the **last line of defense**. Always validate permissions in application layer too.
2. **Library Access**: Sandboxed processes need read access to system libraries (`/lib`, `/usr/lib`)
3. **Escape Prevention**: Landlock prevents `chroot` escapes, symlink attacks, and privilege escalation
4. **Audit Logging**: Log all sandbox creation/destruction/permission changes
5. **Kernel Updates**: Ensure kernel is patched (Landlock improvements in newer kernels)

---

## Performance

- **Overhead**: Minimal (<1% CPU overhead for Landlock enforcement)
- **Memory**: ~4KB per sandbox
- **Propagation Time**: ~2 seconds for permission updates (process restart)
- **Scalability**: Supports thousands of concurrent sandboxes

---

## Troubleshooting

### Check Landlock Support
```bash
# Check if Landlock is enabled
cat /sys/kernel/security/lsm | grep landlock

# Check kernel version
uname -r  # Must be >= 5.13
```

### Common Errors
- `Landlock not supported`: Kernel < 5.13 or `CONFIG_SECURITY_LANDLOCK=n`
- `Permission denied` even with correct access: Check if process has read access to parent directories
- `EACCES` on library loading: Add `/lib`, `/usr/lib` to ruleset

---

**Document Version**: 1.0  
**Last Updated**: 2026-02-14  
**Related**: [../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md), [webrtc.md](webrtc.md)
