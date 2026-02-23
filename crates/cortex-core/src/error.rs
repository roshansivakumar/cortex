use thiserror::Error;

#[derive(Error, Debug)]
pub enum CortexError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CortexError>;
