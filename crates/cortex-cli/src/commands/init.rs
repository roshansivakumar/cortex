use cortex_core::config::CortexConfig;
use cortex_core::error::Result;

pub async fn run() -> Result<()> {
    let config = CortexConfig::default();

    // Create data directory
    std::fs::create_dir_all(&config.storage.data_dir)?;
    println!("Created data directory: {}", config.storage.data_dir.display());

    // Create model directory
    std::fs::create_dir_all(&config.model_dir())?;

    // Download model
    println!("Downloading embedding model: {}...", config.embedding.model);
    cortex_embed::model::ensure_model(&config.model_dir(), &config.embedding.model)?;
    println!("Model ready.");

    // Write default config if it doesn't exist
    if !config.config_path().exists() {
        config.save()?;
        println!("Config written to: {}", config.config_path().display());
    } else {
        println!("Config already exists: {}", config.config_path().display());
    }

    println!("\nCortex initialized at {}", config.storage.data_dir.display());
    println!("Next: cortex start --watch ~/notes");

    Ok(())
}
