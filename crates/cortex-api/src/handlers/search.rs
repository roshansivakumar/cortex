use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use cortex_core::traits::{ChunkStore, DocumentStore, VectorStore};

use crate::server::AppState;
use crate::types::{SearchHit, SearchRequest, SearchResponse};

pub async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    // 1. Embed the query
    let query_text = req.query.clone();
    let embedder = state.embedder.clone();
    let store = state.store.clone();
    let limit = req.limit;
    let min_score = req.min_score;

    let result = tokio::task::spawn_blocking(move || -> Result<SearchResponse, String> {
        // Embed query
        let vectors = embedder
            .embed(&[query_text.as_str()])
            .map_err(|e| format!("Embedding error: {e}"))?;

        let query_vector = vectors
            .into_iter()
            .next()
            .ok_or_else(|| "No embedding produced".to_string())?;

        // Search vectors
        let results = store
            .search(&query_vector, limit)
            .map_err(|e| format!("Search error: {e}"))?;

        // Fetch chunk + document details for each hit
        let mut hits = Vec::new();
        for (chunk_id, score) in results {
            if let Some(min) = min_score {
                if score < min {
                    continue;
                }
            }

            // Find the chunk
            // We need to look up which document this chunk belongs to
            // For now, query all documents and match
            let documents = store
                .list_documents()
                .map_err(|e| format!("List error: {e}"))?;

            for (doc_id, doc_meta, source_path) in &documents {
                let chunks = store
                    .get_chunks_for_document(doc_id)
                    .map_err(|e| format!("Chunk error: {e}"))?;

                for chunk in &chunks {
                    if chunk.id == chunk_id {
                        hits.push(SearchHit {
                            chunk_id: chunk.id.clone(),
                            document_id: doc_id.clone(),
                            source_path: source_path.clone(),
                            title: doc_meta.title.clone(),
                            content: chunk.content.clone(),
                            score,
                        });
                    }
                }
            }
        }

        Ok(SearchResponse {
            total: hits.len(),
            results: hits,
            query: query_text.clone(),
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}
