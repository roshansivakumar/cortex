use cortex_core::error::{CortexError, Result};

use crate::connection::SqliteStore;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    source_path TEXT UNIQUE NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    file_type TEXT NOT NULL,
    title TEXT,
    source_plugin TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS chunks (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    byte_offset INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL,
    token_count INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id);

CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    content,
    content='chunks',
    content_rowid='rowid'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
    INSERT INTO chunks_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

-- Vector embeddings table
-- NOTE: sqlite-vec virtual table will be created separately when available.
-- For MVP, we store vectors as blobs and do brute-force cosine similarity.
CREATE TABLE IF NOT EXISTS chunk_embeddings (
    chunk_id TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
    embedding BLOB NOT NULL
);
"#;

pub fn run(store: &SqliteStore) -> Result<()> {
    let conn = store.conn();
    conn.execute_batch(SCHEMA)
        .map_err(|e| CortexError::Database(format!("Migration failed: {e}")))?;
    Ok(())
}
