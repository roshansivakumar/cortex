use serde::{Deserialize, Serialize};

use crate::document::{Chunk, DocumentMetadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub min_score: Option<f32>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: Chunk,
    pub document: DocumentMetadata,
    pub score: f32,
    pub source_path: String,
}
