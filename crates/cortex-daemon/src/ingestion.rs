use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use tokio_util::sync::CancellationToken;

use cortex_core::document::{DocumentEvent, Embedding};
use cortex_core::error::Result;
use cortex_core::traits::{ChunkStore, Chunker, DocumentStore, Embedder, VectorStore};
use cortex_store::connection::SqliteStore;

/// Run the ingestion loop: receive events, debounce, process.
pub async fn ingestion_loop(
    receiver: Receiver<DocumentEvent>,
    store: Arc<SqliteStore>,
    embedder: Arc<dyn Embedder>,
    chunker: Arc<dyn Chunker>,
    plugin: Arc<dyn cortex_core::traits::Plugin>,
    cancel: CancellationToken,
) {
    tracing::info!("Ingestion pipeline started.");

    loop {
        if cancel.is_cancelled() {
            break;
        }

        // Collect events with debouncing (500ms window)
        let mut events: HashMap<PathBuf, DocumentEvent> = HashMap::new();

        // Block waiting for first event
        match receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
                let path = event_path(&event).to_path_buf();
                events.insert(path, event);
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        // Drain any additional events that arrived during the debounce window
        std::thread::sleep(Duration::from_millis(200));
        while let Ok(event) = receiver.try_recv() {
            let path = event_path(&event).to_path_buf();
            events.insert(path, event);
        }

        // Process deduplicated events
        for (path, event) in events {
            let store = store.clone();
            let embedder = embedder.clone();
            let chunker = chunker.clone();
            let plugin = plugin.clone();

            if let Err(e) = process_event(&path, &event, &store, &embedder, &chunker, &plugin) {
                tracing::error!("Failed to process {}: {e}", path.display());
            }
        }
    }

    tracing::info!("Ingestion pipeline stopped.");
}

fn event_path(event: &DocumentEvent) -> &std::path::Path {
    match event {
        DocumentEvent::Created(p) | DocumentEvent::Modified(p) | DocumentEvent::Deleted(p) => p,
    }
}

fn process_event(
    path: &std::path::Path,
    event: &DocumentEvent,
    store: &Arc<SqliteStore>,
    embedder: &Arc<dyn Embedder>,
    chunker: &Arc<dyn Chunker>,
    plugin: &Arc<dyn cortex_core::traits::Plugin>,
) -> Result<()> {
    match event {
        DocumentEvent::Deleted(_) => {
            // Remove document and all associated data
            if let Some(existing) = store.get_document_by_path(path)? {
                tracing::info!("Removing deleted document: {}", path.display());
                store.delete_embeddings_for_document(&existing.id)?;
                store.delete_chunks_for_document(&existing.id)?;
                store.delete_document(&existing.id)?;
            }
        }
        DocumentEvent::Created(_) | DocumentEvent::Modified(_) => {
            // Read the document
            let doc = plugin.read_document(path)?;

            // Check if content has changed
            if let Some(existing) = store.get_document_by_path(path)? {
                if existing.content_hash == doc.content_hash {
                    tracing::debug!("Skipping unchanged file: {}", path.display());
                    return Ok(());
                }
                // Content changed — remove old data
                tracing::info!("Re-indexing changed document: {}", path.display());
                store.delete_embeddings_for_document(&existing.id)?;
                store.delete_chunks_for_document(&existing.id)?;
                store.delete_document(&existing.id)?;
            } else {
                tracing::info!("Indexing new document: {}", path.display());
            }

            // Chunk the document
            let chunks = chunker.chunk(&doc)?;
            if chunks.is_empty() {
                tracing::debug!("No chunks produced for: {}", path.display());
                return Ok(());
            }

            // Embed chunks in batches
            let batch_size = 32;
            let mut all_embeddings = Vec::new();

            for batch in chunks.chunks(batch_size) {
                let texts: Vec<&str> = batch.iter().map(|c| c.content.as_str()).collect();
                let vectors = embedder.embed(&texts)?;

                for (chunk, vector) in batch.iter().zip(vectors) {
                    all_embeddings.push(Embedding {
                        chunk_id: chunk.id.clone(),
                        vector,
                    });
                }
            }

            // Store everything
            store.upsert_document(&doc)?;
            store.insert_chunks(&chunks)?;
            store.insert_embeddings(&all_embeddings)?;

            tracing::info!(
                "Indexed: {} ({} chunks)",
                path.display(),
                chunks.len()
            );
        }
    }

    Ok(())
}
