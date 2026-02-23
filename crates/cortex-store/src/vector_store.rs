use cortex_core::document::{ChunkId, DocumentId, Embedding};
use cortex_core::error::{CortexError, Result};
use cortex_core::traits::VectorStore;

use crate::connection::SqliteStore;

impl VectorStore for SqliteStore {
    fn insert_embeddings(&self, embeddings: &[Embedding]) -> Result<()> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "INSERT INTO chunk_embeddings (chunk_id, embedding) VALUES (?1, ?2)",
            )
            .map_err(|e| CortexError::Database(e.to_string()))?;

        for emb in embeddings {
            let blob = vectors_to_blob(&emb.vector);
            stmt.execute(rusqlite::params![emb.chunk_id, blob])
                .map_err(|e| CortexError::Database(e.to_string()))?;
        }

        Ok(())
    }

    fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<(ChunkId, f32)>> {
        let conn = self.conn();

        // Brute-force cosine similarity over all stored vectors.
        // At personal scale (<100K chunks) this completes in milliseconds.
        let mut stmt = conn
            .prepare("SELECT chunk_id, embedding FROM chunk_embeddings")
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let chunk_id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((chunk_id, blob))
            })
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut scored: Vec<(ChunkId, f32)> = Vec::new();
        for row in rows {
            let (chunk_id, blob) = row.map_err(|e| CortexError::Database(e.to_string()))?;
            let stored_vector = blob_to_vectors(&blob);
            let score = cosine_similarity(query_vector, &stored_vector);
            scored.push((chunk_id, score));
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }

    fn delete_embeddings_for_document(&self, doc_id: &DocumentId) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM chunk_embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE document_id = ?1)",
            rusqlite::params![doc_id],
        )
        .map_err(|e| CortexError::Database(e.to_string()))?;
        Ok(())
    }
}

/// Serialize f32 vector to bytes (little-endian).
fn vectors_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for &v in vector {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

/// Deserialize bytes back to f32 vector.
fn blob_to_vectors(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_roundtrip() {
        let original = vec![1.0f32, 2.0, 3.0, -1.5, 0.0];
        let blob = vectors_to_blob(&original);
        let recovered = blob_to_vectors(&blob);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }
}
