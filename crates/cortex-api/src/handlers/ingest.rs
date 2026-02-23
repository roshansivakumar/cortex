use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::server::AppState;
use crate::types::{IngestRequest, MessageResponse};

pub async fn ingest(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<MessageResponse>, StatusCode> {
    use cortex_core::document::DocumentEvent;
    use std::path::PathBuf;

    let path = PathBuf::from(&req.path);

    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Send a re-index event through the ingestion channel
    state
        .ingest_sender
        .send(DocumentEvent::Modified(path))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(MessageResponse {
        message: format!("Queued for re-indexing: {}", req.path),
    }))
}
