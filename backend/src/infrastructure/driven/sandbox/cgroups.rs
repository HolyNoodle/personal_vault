use std::path::PathBuf;
use tracing::{info, warn};

const CGROUP_BASE: &str = "/sys/fs/cgroup/sandbox";

fn cgroup_path(session_id: &str) -> PathBuf {
    PathBuf::from(CGROUP_BASE).join(session_id)
}

/// Create a cgroup v2 for the given session and configure resource limits.
/// Must be called in the PARENT process after spawning the child.
pub fn setup_cgroup(session_id: &str, pid: u32) -> std::io::Result<()> {
    let dir = cgroup_path(session_id);

    // Ensure parent cgroup exists
    if let Err(e) = std::fs::create_dir_all(&dir) {
        warn!("cgroup: failed to create {}: {}", dir.display(), e);
        return Ok(()); // non-fatal: app runs without cgroup limits
    }

    // Write PID into cgroup
    std::fs::write(dir.join("cgroup.procs"), pid.to_string())
        .unwrap_or_else(|e| warn!("cgroup: failed to write cgroup.procs: {}", e));

    // CPU: 50% of one core (500ms per 1000ms period)
    std::fs::write(dir.join("cpu.max"), "500000 1000000")
        .unwrap_or_else(|e| warn!("cgroup: failed to set cpu.max: {}", e));

    // Memory: 512 MiB
    std::fs::write(dir.join("memory.max"), (512 * 1024 * 1024).to_string())
        .unwrap_or_else(|e| warn!("cgroup: failed to set memory.max: {}", e));

    // PID count: max 100 threads/processes
    std::fs::write(dir.join("pids.max"), "100")
        .unwrap_or_else(|e| warn!("cgroup: failed to set pids.max: {}", e));

    info!("cgroup v2 configured for session {} (pid {})", session_id, pid);
    Ok(())
}

/// Remove the cgroup for a session (called on cleanup).
pub fn teardown_cgroup(session_id: &str) {
    let dir = cgroup_path(session_id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir(&dir) {
            warn!("cgroup: failed to remove {}: {}", dir.display(), e);
        } else {
            info!("cgroup removed for session {}", session_id);
        }
    }
}
