use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use cortex_core::traits::{DocumentStore, VectorStore};

use crate::server::AppState;
use crate::types::{DocumentResponse, MessageResponse};

pub async fn list_documents(
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentResponse>>, StatusCode> {
    let store = state.store.clone();

    let result = tokio::task::spawn_blocking(move || {
        store.list_documents().map(|docs| {
            docs.into_iter()
                .map(|(id, meta, source_path)| DocumentResponse {
                    id,
                    source_path,
                    title: meta.title,
                    file_type: meta.file_type.as_str().to_string(),
                    size_bytes: meta.size_bytes,
                })
                .collect::<Vec<_>>()
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

pub async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MessageResponse>, StatusCode> {
    let store = state.store.clone();

    tokio::task::spawn_blocking(move || {
        // Delete embeddings, chunks, then document (cascading delete handles chunks)
        store.delete_embeddings_for_document(&id)?;
        store.delete_document(&id)?;
        Ok::<_, cortex_core::error::CortexError>(())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(MessageResponse {
        message: "Document deleted".to_string(),
    }))
}
