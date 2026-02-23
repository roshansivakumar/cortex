use cortex_core::config::CortexConfig;
use cortex_core::error::Result;

use crate::client;

pub async fn run() -> Result<()> {
    let config = CortexConfig::load();

    let response = client::get(&config.socket_path(), "/status").await?;
    let status: serde_json::Value = serde_json::from_str(&response)?;

    println!("Cortex Status");
    println!("─────────────────────────────");

    if let Some(s) = status.get("status").and_then(|v| v.as_str()) {
        println!("  Status:     {s}");
    }
    if let Some(u) = status.get("uptime_secs").and_then(|v| v.as_u64()) {
        let hours = u / 3600;
        let mins = (u % 3600) / 60;
        let secs = u % 60;
        println!("  Uptime:     {hours}h {mins}m {secs}s");
    }
    if let Some(d) = status.get("document_count").and_then(|v| v.as_u64()) {
        println!("  Documents:  {d}");
    }
    if let Some(c) = status.get("chunk_count").and_then(|v| v.as_u64()) {
        println!("  Chunks:     {c}");
    }
    if let Some(m) = status.get("model").and_then(|v| v.as_str()) {
        println!("  Model:      {m}");
    }
    if let Some(paths) = status.get("watched_paths").and_then(|v| v.as_array()) {
        println!("  Watching:");
        for p in paths {
            if let Some(s) = p.as_str() {
                println!("    - {s}");
            }
        }
    }

    Ok(())
}
