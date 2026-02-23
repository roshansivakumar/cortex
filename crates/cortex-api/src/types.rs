use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub min_score: Option<f32>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchHit>,
    pub query: String,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub chunk_id: String,
    pub document_id: String,
    pub source_path: String,
    pub title: Option<String>,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub document_count: usize,
    pub chunk_count: usize,
    pub watched_paths: Vec<String>,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub id: String,
    pub source_path: String,
    pub title: Option<String>,
    pub file_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}
