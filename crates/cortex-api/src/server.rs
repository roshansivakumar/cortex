use std::sync::Arc;
use std::time::Instant;

use axum::routing::{delete, get, post};
use axum::Router;

use cortex_core::config::CortexConfig;
use cortex_core::document::DocumentEvent;
use cortex_store::connection::SqliteStore;

use crate::handlers;

/// Shared application state for all handlers.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<SqliteStore>,
    pub embedder: Arc<dyn cortex_core::traits::Embedder>,
    pub config: Arc<CortexConfig>,
    pub start_time: Instant,
    pub ingest_sender: crossbeam_channel::Sender<DocumentEvent>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/search", post(handlers::search::search))
        .route("/documents", get(handlers::documents::list_documents))
        .route("/documents/{id}", delete(handlers::documents::delete_document))
        .route("/ingest", post(handlers::ingest::ingest))
        .route("/status", get(handlers::status::status))
        .with_state(state)
}
