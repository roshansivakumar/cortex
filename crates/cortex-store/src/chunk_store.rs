use cortex_core::document::{Chunk, DocumentId};
use cortex_core::error::{CortexError, Result};
use cortex_core::traits::ChunkStore;

use crate::connection::SqliteStore;

impl ChunkStore for SqliteStore {
    fn insert_chunks(&self, chunks: &[Chunk]) -> Result<()> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "INSERT INTO chunks (id, document_id, content, byte_offset, chunk_index, token_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| CortexError::Database(e.to_string()))?;

        for chunk in chunks {
            stmt.execute(rusqlite::params![
                chunk.id,
                chunk.document_id,
                chunk.content,
                chunk.byte_offset as i64,
                chunk.chunk_index as i64,
                chunk.token_count as i64,
            ])
            .map_err(|e| CortexError::Database(e.to_string()))?;
        }

        Ok(())
    }

    fn get_chunks_for_document(&self, doc_id: &DocumentId) -> Result<Vec<Chunk>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, document_id, content, byte_offset, chunk_index, token_count
                 FROM chunks WHERE document_id = ?1 ORDER BY chunk_index",
            )
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![doc_id], |row| {
                Ok(Chunk {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    content: row.get(2)?,
                    byte_offset: row.get::<_, i64>(3)? as usize,
                    chunk_index: row.get::<_, i64>(4)? as u32,
                    token_count: row.get::<_, i64>(5)? as usize,
                })
            })
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| CortexError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    fn delete_chunks_for_document(&self, doc_id: &DocumentId) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM chunks WHERE document_id = ?1",
            rusqlite::params![doc_id],
        )
        .map_err(|e| CortexError::Database(e.to_string()))?;
        Ok(())
    }

    fn chunk_count(&self) -> Result<usize> {
        let conn = self.conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))
            .map_err(|e| CortexError::Database(e.to_string()))?;
        Ok(count as usize)
    }
}
