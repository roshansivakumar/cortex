use std::path::PathBuf;

use clap::Args;

use cortex_core::config::CortexConfig;
use cortex_core::error::Result;
use cortex_daemon::daemon::Daemon;

#[derive(Args)]
pub struct StartArgs {
    /// Directories to watch for files
    #[arg(long, value_delimiter = ',')]
    pub watch: Vec<PathBuf>,

    /// Optional TCP port for the API (in addition to Unix socket)
    #[arg(long)]
    pub tcp_port: Option<u16>,
}

pub async fn run(args: StartArgs) -> Result<()> {
    let mut config = CortexConfig::load();

    // Merge CLI args into config
    if !args.watch.is_empty() {
        // Expand ~ in paths
        let expanded: Vec<PathBuf> = args
            .watch
            .iter()
            .map(|p| {
                let s = p.to_string_lossy();
                if s.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&s[2..])
                    } else {
                        p.clone()
                    }
                } else {
                    p.clone()
                }
            })
            .collect();
        config.watch.paths = expanded;
    }

    if let Some(port) = args.tcp_port {
        config.api.tcp_port = Some(port);
    }

    if config.watch.paths.is_empty() {
        eprintln!("No watch paths configured. Use --watch <path> or set paths in config.toml");
        std::process::exit(1);
    }

    println!("Starting Cortex daemon...");
    for path in &config.watch.paths {
        println!("  Watching: {}", path.display());
    }

    let daemon = Daemon::new(config);
    daemon.run().await
}
