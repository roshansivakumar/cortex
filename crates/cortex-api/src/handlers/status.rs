use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use cortex_core::traits::{ChunkStore, DocumentStore};

use crate::server::AppState;
use crate::types::StatusResponse;

pub async fn status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let store = state.store.clone();
    let uptime = state.start_time.elapsed().as_secs();
    let watched_paths = state
        .config
        .watch
        .paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let model = state.config.embedding.model.clone();

    let (doc_count, chunk_count) = tokio::task::spawn_blocking(move || {
        let docs = store.document_count().unwrap_or(0);
        let chunks = store.chunk_count().unwrap_or(0);
        (docs, chunks)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StatusResponse {
        status: "running".to_string(),
        uptime_secs: uptime,
        document_count: doc_count,
        chunk_count: chunk_count,
        watched_paths,
        model,
    }))
}
