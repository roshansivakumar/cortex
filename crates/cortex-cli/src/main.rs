mod commands;
mod client;

use clap::Parser;

#[derive(Parser)]
#[command(name = "cortex", about = "Local-first personal context engine")]
enum Cli {
    /// Initialize Cortex (create data directory, download model)
    Init,
    /// Start the Cortex daemon
    Start(commands::start::StartArgs),
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
    /// Search indexed content
    Search(commands::search::SearchArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli {
        Cli::Init => commands::init::run().await,
        Cli::Start(args) => commands::start::run(args).await,
        Cli::Stop => commands::stop::run().await,
        Cli::Status => commands::status::run().await,
        Cli::Search(args) => commands::search::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
