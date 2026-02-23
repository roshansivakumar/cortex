use clap::Args;

use cortex_core::config::CortexConfig;
use cortex_core::error::Result;

use crate::client;

#[derive(Args)]
pub struct SearchArgs {
    /// The search query
    pub query: String,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: usize,
}

pub async fn run(args: SearchArgs) -> Result<()> {
    let config = CortexConfig::load();

    let body = serde_json::json!({
        "query": args.query,
        "limit": args.limit,
    });

    let response = client::post(
        &config.socket_path(),
        "/search",
        &body.to_string(),
    )
    .await?;

    let result: serde_json::Value = serde_json::from_str(&response)?;

    let total = result
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    if total == 0 {
        println!("No results found for: \"{}\"", args.query);
        return Ok(());
    }

    println!(
        "Found {total} result{} for: \"{}\"\n",
        if total == 1 { "" } else { "s" },
        args.query
    );

    if let Some(results) = result.get("results").and_then(|v| v.as_array()) {
        for (i, hit) in results.iter().enumerate() {
            let score = hit.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let source = hit
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let title = hit
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            let content = hit
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            println!("{}. {} (score: {:.3})", i + 1, title, score);
            println!("   {source}");

            // Show a preview (first 200 chars)
            let preview: String = content.chars().take(200).collect();
            let preview = preview.replace('\n', " ");
            println!("   {preview}");
            println!();
        }
    }

    Ok(())
}
