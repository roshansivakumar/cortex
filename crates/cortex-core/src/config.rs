use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub chunking: ChunkingConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub watch: WatchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_overlap_tokens")]
    pub overlap_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub tcp_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    #[serde(default)]
    pub paths: Vec<PathBuf>,
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,
}

// --- Defaults ---

fn default_log_level() -> String {
    "info".to_string()
}

fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex")
}

fn default_model() -> String {
    "all-MiniLM-L6-v2".to_string()
}

fn default_batch_size() -> usize {
    32
}

fn default_max_tokens() -> usize {
    256
}

fn default_overlap_tokens() -> usize {
    64
}

fn default_extensions() -> Vec<String> {
    vec![
        ".md", ".txt", ".rs", ".py", ".js", ".ts", ".go", ".toml", ".yaml", ".yml", ".json",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_ignore_patterns() -> Vec<String> {
    vec!["node_modules", ".git", "target", "__pycache__", ".venv"]
        .into_iter()
        .map(String::from)
        .collect()
}

// --- Impl ---

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            storage: StorageConfig::default(),
            embedding: EmbeddingConfig::default(),
            chunking: ChunkingConfig::default(),
            api: ApiConfig::default(),
            watch: WatchConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            batch_size: default_batch_size(),
        }
    }
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            overlap_tokens: default_overlap_tokens(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self { tcp_port: None }
    }
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            extensions: default_extensions(),
            ignore_patterns: default_ignore_patterns(),
        }
    }
}

impl CortexConfig {
    /// Resolved path to the SQLite database.
    pub fn db_path(&self) -> PathBuf {
        self.storage.data_dir.join("cortex.db")
    }

    /// Resolved path to the model directory.
    pub fn model_dir(&self) -> PathBuf {
        self.storage.data_dir.join("models").join(&self.embedding.model)
    }

    /// Resolved path to the Unix socket.
    pub fn socket_path(&self) -> PathBuf {
        self.storage.data_dir.join("cortex.sock")
    }

    /// Resolved path to the PID file.
    pub fn pid_path(&self) -> PathBuf {
        self.storage.data_dir.join("cortex.pid")
    }

    /// Resolved path to the config file.
    pub fn config_path(&self) -> PathBuf {
        self.storage.data_dir.join("config.toml")
    }

    /// Load config from the default location, or return defaults.
    pub fn load() -> Self {
        let default_path = default_data_dir().join("config.toml");
        Self::load_from(&default_path).unwrap_or_default()
    }

    /// Load config from a specific path.
    pub fn load_from(path: &std::path::Path) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(crate::error::CortexError::Io)?;
        toml::from_str(&content).map_err(|e| crate::error::CortexError::Config(e.to_string()))
    }

    /// Save config to the default location.
    pub fn save(&self) -> crate::error::Result<()> {
        let path = self.config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| crate::error::CortexError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
