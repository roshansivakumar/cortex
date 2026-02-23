use std::path::Path;

use crossbeam_channel::Sender;

use crate::document::{Chunk, ChunkId, Document, DocumentEvent, DocumentId, DocumentMetadata, Embedding};
use crate::error::Result;

// ---------------------------------------------------------------------------
// Plugin trait — implemented by ingestion plugins (e.g. fs-watcher)
// Plugins run on their own thread (sync). They push events to the daemon.
// ---------------------------------------------------------------------------

pub trait Plugin: Send + Sync + 'static {
    fn name(&self) -> &str;

    /// Start watching and sending events. Runs on a dedicated thread.
    fn start(&mut self, sender: Sender<DocumentEvent>) -> Result<()>;

    /// Gracefully stop.
    fn stop(&mut self) -> Result<()>;

    /// Read a file at the given path and return a Document.
    fn read_document(&self, path: &Path) -> Result<Document>;
}

// ---------------------------------------------------------------------------
// Chunker trait — splits documents into embeddable chunks
// ---------------------------------------------------------------------------

pub trait Chunker: Send + Sync {
    fn chunk(&self, document: &Document) -> Result<Vec<Chunk>>;
}

// ---------------------------------------------------------------------------
// Embedder trait — produces vector embeddings from text
// ---------------------------------------------------------------------------

pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
}

// ---------------------------------------------------------------------------
// Store traits — persistence layer
// ---------------------------------------------------------------------------

pub trait DocumentStore: Send + Sync {
    fn upsert_document(&self, doc: &Document) -> Result<()>;
    fn get_document(&self, id: &DocumentId) -> Result<Option<Document>>;
    fn get_document_by_path(&self, path: &Path) -> Result<Option<Document>>;
    fn delete_document(&self, id: &DocumentId) -> Result<()>;
    fn list_documents(&self) -> Result<Vec<(DocumentId, DocumentMetadata, String)>>;
    fn document_count(&self) -> Result<usize>;
}

pub trait ChunkStore: Send + Sync {
    fn insert_chunks(&self, chunks: &[Chunk]) -> Result<()>;
    fn get_chunks_for_document(&self, doc_id: &DocumentId) -> Result<Vec<Chunk>>;
    fn delete_chunks_for_document(&self, doc_id: &DocumentId) -> Result<()>;
    fn chunk_count(&self) -> Result<usize>;
}

pub trait VectorStore: Send + Sync {
    fn insert_embeddings(&self, embeddings: &[Embedding]) -> Result<()>;
    fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<(ChunkId, f32)>>;
    fn delete_embeddings_for_document(&self, doc_id: &DocumentId) -> Result<()>;
}
