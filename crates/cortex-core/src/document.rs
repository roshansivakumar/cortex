use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

pub type DocumentId = String;
pub type ChunkId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Markdown,
    PlainText,
    Code(String),
    Unknown,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "md" | "markdown" => FileType::Markdown,
            "txt" | "text" => FileType::PlainText,
            "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "java" | "rb" | "sh"
            | "toml" | "yaml" | "yml" | "json" => FileType::Code(ext.to_string()),
            _ => FileType::Unknown,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            FileType::Markdown => "markdown",
            FileType::PlainText => "plaintext",
            FileType::Code(lang) => lang,
            FileType::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub file_type: FileType,
    pub modified_at: SystemTime,
    pub size_bytes: u64,
    pub source_plugin: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: DocumentId,
    pub source_path: PathBuf,
    pub content: String,
    pub content_hash: String,
    pub metadata: DocumentMetadata,
}

impl Document {
    pub fn compute_hash(content: &str) -> String {
        blake3::hash(content.as_bytes()).to_hex().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub document_id: DocumentId,
    pub content: String,
    pub byte_offset: usize,
    pub chunk_index: u32,
    pub token_count: usize,
}

#[derive(Debug, Clone)]
pub struct Embedding {
    pub chunk_id: ChunkId,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub enum DocumentEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}
