use cortex_core::config::CortexConfig;
use cortex_core::error::{CortexError, Result};
use cortex_daemon::lifecycle;

pub async fn run() -> Result<()> {
    let config = CortexConfig::load();
    let pid_path = config.pid_path();

    let pid = lifecycle::read_pid_file(&pid_path)
        .ok_or_else(|| CortexError::Config("No running daemon found (PID file missing)".into()))?;

    if !lifecycle::is_process_running(pid) {
        // Stale PID file
        lifecycle::remove_pid_file(&pid_path);
        println!("Daemon is not running (stale PID file removed).");
        return Ok(());
    }

    // Send SIGTERM
    println!("Stopping Cortex daemon (pid={pid})...");
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }

    // Wait a moment for graceful shutdown
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    if lifecycle::is_process_running(pid) {
        println!("Daemon still running. You may need to kill it manually: kill {pid}");
    } else {
        println!("Daemon stopped.");
    }

    Ok(())
}
