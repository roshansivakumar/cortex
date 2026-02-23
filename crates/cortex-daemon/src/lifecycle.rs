use std::path::Path;

use cortex_core::error::Result;

/// Write the current process PID to a file.
pub fn write_pid_file(path: &Path) -> Result<()> {
    let pid = std::process::id();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, pid.to_string())?;
    tracing::info!("PID file written: {} (pid={})", path.display(), pid);
    Ok(())
}

/// Remove the PID file.
pub fn remove_pid_file(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
        tracing::info!("PID file removed: {}", path.display());
    }
}

/// Remove the Unix socket file.
pub fn remove_socket_file(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
        tracing::info!("Socket file removed: {}", path.display());
    }
}

/// Read the PID from the PID file, if it exists.
pub fn read_pid_file(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check if a process with the given PID is running.
pub fn is_process_running(pid: u32) -> bool {
    // On Unix, kill(pid, 0) checks if process exists without sending a signal
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
