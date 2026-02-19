use landlock::{
    Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI,
};

/// Apply Landlock filesystem restrictions in the child process (called from pre_exec).
///
/// - `root_path`: owner's storage root (read/write/delete access)
/// - `allowed_paths`: client-specific allowed paths (overrides root_path when non-empty)
///
/// Also grants read-only access to system paths required for the app to run.
pub fn apply_landlock(root_path: &str, allowed_paths: &[String]) -> std::io::Result<()> {
    if root_path.is_empty() && allowed_paths.is_empty() {
        return Ok(());
    }

    let abi = ABI::V3;
    let access_all = AccessFs::from_all(abi);
    // Read + execute access for system paths (no write/delete)
    let access_read = AccessFs::ReadFile | AccessFs::ReadDir | AccessFs::Execute;

    let ruleset = Ruleset::default()
        .handle_access(access_all)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Landlock handle_access: {e}")))?
        .create()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Landlock create: {e}")))?;

    let mut ruleset = ruleset;

    // System paths: read/execute only
    let system_dirs = ["/usr", "/lib", "/lib64", "/lib32", "/etc/fonts", "/proc/self", "/dev"];
    for dir in &system_dirs {
        if std::path::Path::new(dir).exists() {
            if let Ok(fd) = PathFd::new(dir) {
                ruleset = ruleset
                    .add_rule(PathBeneath::new(fd, access_read))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Landlock add system rule: {e}")))?;
            }
        }
    }

    // X11 socket dir: read+write for AF_UNIX socket connections
    let x11_dir = "/tmp/.X11-unix";
    if std::path::Path::new(x11_dir).exists() {
        if let Ok(fd) = PathFd::new(x11_dir) {
            ruleset = ruleset
                .add_rule(PathBeneath::new(fd, access_read))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Landlock add X11 rule: {e}")))?;
        }
    }

    // User data paths: full access
    let data_paths: Vec<&str> = if !allowed_paths.is_empty() {
        allowed_paths.iter().map(|s| s.as_str()).collect()
    } else {
        vec![root_path]
    };

    for path in data_paths {
        if std::path::Path::new(path).exists() {
            if let Ok(fd) = PathFd::new(path) {
                ruleset = ruleset
                    .add_rule(PathBeneath::new(fd, access_all))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Landlock add data rule: {e}")))?;
            }
        }
    }

    ruleset
        .restrict_self()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Landlock restrict_self: {e}")))?;

    Ok(())
}
