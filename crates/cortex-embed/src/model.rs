use std::path::{Path, PathBuf};

use cortex_core::error::{CortexError, Result};

const HF_BASE_URL: &str = "https://huggingface.co/sentence-transformers";

/// Ensure the ONNX model and tokenizer files exist in the model directory.
/// Downloads them from Hugging Face if missing.
pub fn ensure_model(model_dir: &Path, model_name: &str) -> Result<ModelPaths> {
    std::fs::create_dir_all(model_dir)?;

    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    if !model_path.exists() {
        tracing::info!("Downloading ONNX model: {model_name}...");
        let url = format!(
            "{HF_BASE_URL}/{model_name}/resolve/main/onnx/model.onnx"
        );
        download_file(&url, &model_path)?;
        tracing::info!("Model downloaded to {}", model_path.display());
    }

    if !tokenizer_path.exists() {
        tracing::info!("Downloading tokenizer for: {model_name}...");
        let url = format!(
            "{HF_BASE_URL}/{model_name}/resolve/main/tokenizer.json"
        );
        download_file(&url, &tokenizer_path)?;
        tracing::info!("Tokenizer downloaded to {}", tokenizer_path.display());
    }

    Ok(ModelPaths {
        model: model_path,
        tokenizer: tokenizer_path,
    })
}

pub struct ModelPaths {
    pub model: PathBuf,
    pub tokenizer: PathBuf,
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::blocking::get(url)
        .map_err(|e| CortexError::ModelNotFound(format!("Download failed: {e}")))?;

    if !response.status().is_success() {
        return Err(CortexError::ModelNotFound(format!(
            "HTTP {}: {}",
            response.status(),
            url
        )));
    }

    let bytes = response
        .bytes()
        .map_err(|e| CortexError::ModelNotFound(format!("Failed to read response: {e}")))?;

    std::fs::write(dest, &bytes)?;
    Ok(())
}
