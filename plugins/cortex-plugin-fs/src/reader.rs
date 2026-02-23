use std::path::Path;
use std::time::SystemTime;

use cortex_core::document::{Document, DocumentMetadata, FileType};
use cortex_core::error::{CortexError, Result};

/// Read a file from disk and produce a Document.
pub fn read_file(path: &Path, plugin_name: &str) -> Result<Document> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        CortexError::Plugin(format!("Failed to read {}: {e}", path.display()))
    })?;

    let metadata = std::fs::metadata(path)?;

    let file_type = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(FileType::from_extension)
        .unwrap_or(FileType::Unknown);

    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from);

    let content_hash = Document::compute_hash(&content);
    let id = uuid::Uuid::now_v7().to_string();

    Ok(Document {
        id,
        source_path: path.to_path_buf(),
        content_hash,
        content,
        metadata: DocumentMetadata {
            title,
            file_type,
            modified_at: metadata.modified().unwrap_or(SystemTime::now()),
            size_bytes: metadata.len(),
            source_plugin: plugin_name.to_string(),
        },
    })
}

/// Check if a path has an allowed extension.
pub fn has_allowed_extension(path: &Path, extensions: &[String]) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => format!(".{e}"),
        None => return false,
    };
    extensions.iter().any(|allowed| allowed == &ext)
}

/// Check if a path contains any of the ignore patterns.
pub fn should_ignore(path: &Path, ignore_patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    ignore_patterns
        .iter()
        .any(|pattern| path_str.contains(pattern))
}
