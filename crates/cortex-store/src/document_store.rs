use std::path::Path;
use std::time::UNIX_EPOCH;

use cortex_core::document::{Document, DocumentId, DocumentMetadata, FileType};
use cortex_core::error::{CortexError, Result};
use cortex_core::traits::DocumentStore;

use crate::connection::SqliteStore;

impl DocumentStore for SqliteStore {
    fn upsert_document(&self, doc: &Document) -> Result<()> {
        let conn = self.conn();
        let modified_secs = doc
            .metadata
            .modified_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO documents (id, source_path, content, content_hash, file_type, title, source_plugin, size_bytes, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(source_path) DO UPDATE SET
                 content = excluded.content,
                 content_hash = excluded.content_hash,
                 file_type = excluded.file_type,
                 title = excluded.title,
                 size_bytes = excluded.size_bytes,
                 modified_at = excluded.modified_at,
                 indexed_at = unixepoch()",
            rusqlite::params![
                doc.id,
                doc.source_path.to_string_lossy().as_ref(),
                doc.content,
                doc.content_hash,
                doc.metadata.file_type.as_str(),
                doc.metadata.title,
                doc.metadata.source_plugin,
                doc.metadata.size_bytes as i64,
                modified_secs,
            ],
        )
        .map_err(|e| CortexError::Database(e.to_string()))?;

        Ok(())
    }

    fn get_document(&self, id: &DocumentId) -> Result<Option<Document>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, source_path, content, content_hash, file_type, title, source_plugin, size_bytes, modified_at
                 FROM documents WHERE id = ?1",
            )
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                Ok(row_to_document(row))
            })
            .optional()
            .map_err(|e| CortexError::Database(e.to_string()))?;

        match result {
            Some(doc) => Ok(Some(doc)),
            None => Ok(None),
        }
    }

    fn get_document_by_path(&self, path: &Path) -> Result<Option<Document>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, source_path, content, content_hash, file_type, title, source_plugin, size_bytes, modified_at
                 FROM documents WHERE source_path = ?1",
            )
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let path_str = path.to_string_lossy();
        let result = stmt
            .query_row(rusqlite::params![path_str.as_ref()], |row| {
                Ok(row_to_document(row))
            })
            .optional()
            .map_err(|e| CortexError::Database(e.to_string()))?;

        match result {
            Some(doc) => Ok(Some(doc)),
            None => Ok(None),
        }
    }

    fn delete_document(&self, id: &DocumentId) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM documents WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| CortexError::Database(e.to_string()))?;
        Ok(())
    }

    fn list_documents(&self) -> Result<Vec<(DocumentId, DocumentMetadata, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, source_path, file_type, title, source_plugin, size_bytes, modified_at FROM documents ORDER BY modified_at DESC",
            )
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let source_path: String = row.get(1)?;
                let file_type_str: String = row.get(2)?;
                let title: Option<String> = row.get(3)?;
                let source_plugin: String = row.get(4)?;
                let size_bytes: i64 = row.get(5)?;
                let modified_secs: i64 = row.get(6)?;

                let metadata = DocumentMetadata {
                    title,
                    file_type: FileType::from_extension(&file_type_str),
                    modified_at: UNIX_EPOCH + std::time::Duration::from_secs(modified_secs as u64),
                    size_bytes: size_bytes as u64,
                    source_plugin,
                };

                Ok((id, metadata, source_path))
            })
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| CortexError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    fn document_count(&self) -> Result<usize> {
        let conn = self.conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .map_err(|e| CortexError::Database(e.to_string()))?;
        Ok(count as usize)
    }
}

fn row_to_document(row: &rusqlite::Row<'_>) -> Document {
    let id: String = row.get(0).unwrap();
    let source_path: String = row.get(1).unwrap();
    let content: String = row.get(2).unwrap();
    let content_hash: String = row.get(3).unwrap();
    let file_type_str: String = row.get(4).unwrap();
    let title: Option<String> = row.get(5).unwrap();
    let source_plugin: String = row.get(6).unwrap();
    let size_bytes: i64 = row.get(7).unwrap();
    let modified_secs: i64 = row.get(8).unwrap();

    Document {
        id,
        source_path: source_path.into(),
        content,
        content_hash,
        metadata: DocumentMetadata {
            title,
            file_type: FileType::from_extension(&file_type_str),
            modified_at: UNIX_EPOCH + std::time::Duration::from_secs(modified_secs as u64),
            size_bytes: size_bytes as u64,
            source_plugin,
        },
    }
}

use rusqlite::OptionalExtension;
