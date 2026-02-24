use cortex_core::document::{Chunk, Document};
use cortex_core::error::Result;
use cortex_core::traits::Chunker;

/// Splits documents into chunks by paragraph boundaries.
pub struct ParagraphChunker {
    max_chars: usize,
    overlap_chars: usize,
}

impl ParagraphChunker {
    pub fn new(max_tokens: usize, overlap_tokens: usize) -> Self {
        // Rough approximation: 1 token ≈ 4 chars for English text
        Self {
            max_chars: max_tokens * 4,
            overlap_chars: overlap_tokens * 4,
        }
    }
}

impl Chunker for ParagraphChunker {
    fn chunk(&self, document: &Document) -> Result<Vec<Chunk>> {
        let text = &document.content;
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Split by double newline (paragraphs)
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        if paragraphs.is_empty() {
            return Ok(Vec::new());
        }

        // Merge small consecutive paragraphs, split oversized ones
        let mut chunks = Vec::new();
        let mut current_text = String::new();
        let mut current_byte_offset = 0usize;
        let mut chunk_start_offset = 0usize;

        for para in &paragraphs {
            // Find actual byte offset of this paragraph in the original text
            if let Some(pos) = text[current_byte_offset..].find(para) {
                current_byte_offset += pos;
            }

            if current_text.is_empty() {
                chunk_start_offset = current_byte_offset;
                current_text = para.to_string();
            } else if current_text.len() + para.len() + 2 <= self.max_chars {
                // Fits in current chunk
                current_text.push_str("\n\n");
                current_text.push_str(para);
            } else {
                // Flush current chunk
                let chunk_index = chunks.len() as u32;
                chunks.push(make_chunk(
                    &document.id,
                    &current_text,
                    chunk_start_offset,
                    chunk_index,
                ));

                // Start new chunk with overlap (char-safe)
                let overlap_text = safe_tail(&current_text, self.overlap_chars);

                chunk_start_offset = current_byte_offset;
                if overlap_text.is_empty() {
                    current_text = para.to_string();
                } else {
                    current_text = format!("{overlap_text}\n\n{para}");
                }
            }

            current_byte_offset += para.len();
        }

        // Flush remaining
        if !current_text.is_empty() {
            let chunk_index = chunks.len() as u32;
            chunks.push(make_chunk(
                &document.id,
                &current_text,
                chunk_start_offset,
                chunk_index,
            ));
        }

        // Handle oversized chunks by splitting them
        let mut final_chunks = Vec::new();
        let mut idx = 0u32;
        for chunk in chunks {
            if chunk.content.len() > self.max_chars * 2 {
                // Split this chunk further
                let sub_chunks = split_large_text(&document.id, &chunk.content, self.max_chars, &mut idx);
                final_chunks.extend(sub_chunks);
            } else {
                final_chunks.push(Chunk {
                    chunk_index: idx,
                    ..chunk
                });
                idx += 1;
            }
        }

        Ok(final_chunks)
    }
}

fn make_chunk(doc_id: &str, content: &str, byte_offset: usize, chunk_index: u32) -> Chunk {
    // Approximate token count (chars / 4)
    let token_count = content.len() / 4;
    Chunk {
        id: uuid::Uuid::now_v7().to_string(),
        document_id: doc_id.to_string(),
        content: content.to_string(),
        byte_offset,
        chunk_index,
        token_count,
    }
}

/// Get the last `n` chars of a string, starting at a word boundary.
fn safe_tail(text: &str, n: usize) -> &str {
    let char_count = text.chars().count();
    if char_count <= n {
        return text;
    }
    let skip = char_count - n;
    let byte_offset = text.char_indices().nth(skip).map(|(i, _)| i).unwrap_or(0);
    let tail = &text[byte_offset..];
    // Snap to next word boundary
    if let Some(pos) = tail.find(' ') {
        &tail[pos + 1..]
    } else {
        tail
    }
}

fn split_large_text(doc_id: &str, text: &str, max_chars: usize, idx: &mut u32) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let total = chars.len();
    let mut char_start = 0;

    while char_start < total {
        let char_end = (char_start + max_chars).min(total);
        let byte_start = chars[char_start].0;

        // Try to break at a sentence boundary
        let actual_char_end = if char_end < total {
            let byte_end = chars[char_end].0;
            let window = &text[byte_start..byte_end];
            if let Some(p) = window.rfind(". ") {
                // Find which char index corresponds to this byte position
                let sentence_byte = byte_start + p + 2;
                chars[char_start..char_end]
                    .iter()
                    .position(|(b, _)| *b >= sentence_byte)
                    .map(|i| char_start + i)
                    .unwrap_or(char_end)
            } else {
                char_end
            }
        } else {
            char_end
        };

        let byte_end = if actual_char_end < total {
            chars[actual_char_end].0
        } else {
            text.len()
        };

        let slice = &text[byte_start..byte_end];
        if !slice.trim().is_empty() {
            chunks.push(make_chunk(doc_id, slice.trim(), byte_start, *idx));
            *idx += 1;
        }
        char_start = actual_char_end;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::document::{DocumentMetadata, FileType};
    use std::time::SystemTime;

    fn test_doc(content: &str) -> Document {
        Document {
            id: "test-doc".to_string(),
            source_path: "/tmp/test.md".into(),
            content: content.to_string(),
            content_hash: "abc".to_string(),
            metadata: DocumentMetadata {
                title: Some("Test".to_string()),
                file_type: FileType::Markdown,
                modified_at: SystemTime::now(),
                size_bytes: content.len() as u64,
                source_plugin: "test".to_string(),
            },
        }
    }

    #[test]
    fn test_empty_document() {
        let chunker = ParagraphChunker::new(256, 64);
        let doc = test_doc("");
        let chunks = chunker.chunk(&doc).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_single_paragraph() {
        let chunker = ParagraphChunker::new(256, 64);
        let doc = test_doc("Hello world, this is a test paragraph.");
        let chunks = chunker.chunk(&doc).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello world, this is a test paragraph.");
    }

    #[test]
    fn test_multiple_paragraphs() {
        let chunker = ParagraphChunker::new(256, 64);
        let doc = test_doc("First paragraph.\n\nSecond paragraph.\n\nThird paragraph.");
        let chunks = chunker.chunk(&doc).unwrap();
        // All three should fit in one chunk at 256 tokens
        assert_eq!(chunks.len(), 1);
    }
}
