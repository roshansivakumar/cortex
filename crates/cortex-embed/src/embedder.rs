use std::path::Path;
use std::sync::Mutex;

use ndarray::Array2;
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use cortex_core::error::{CortexError, Result};
use cortex_core::traits::Embedder;

use crate::pooling::mean_pool_and_normalize;

pub struct OnnxEmbedder {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    dimension: usize,
}

impl OnnxEmbedder {
    /// Load from a model directory containing model.onnx and tokenizer.json.
    pub fn new(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(CortexError::ModelNotFound(format!(
                "model.onnx not found in {}",
                model_dir.display()
            )));
        }

        if !tokenizer_path.exists() {
            return Err(CortexError::ModelNotFound(format!(
                "tokenizer.json not found in {}",
                model_dir.display()
            )));
        }

        let session = Session::builder()
            .and_then(|b| b.with_intra_threads(4))
            .and_then(|b| b.commit_from_file(&model_path))
            .map_err(|e| CortexError::Embedding(format!("Failed to load ONNX model: {e}")))?;

        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| CortexError::Embedding(format!("Failed to load tokenizer: {e}")))?;

        // Configure truncation and padding
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: 256,
                ..Default::default()
            }))
            .map_err(|e| CortexError::Embedding(format!("Truncation config error: {e}")))?;

        tokenizer.with_padding(Some(tokenizers::PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            dimension: 384, // all-MiniLM-L6-v2 output dim
        })
    }
}

impl Embedder for OnnxEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Tokenize all texts
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| CortexError::Embedding(format!("Tokenization failed: {e}")))?;

        let batch_size = encodings.len();
        let seq_len = encodings[0].get_ids().len();

        // 2. Build input tensors as flat Vec<i64>
        let mut input_ids_flat = Vec::with_capacity(batch_size * seq_len);
        let mut attention_mask_flat = Vec::with_capacity(batch_size * seq_len);
        let mut token_type_ids_flat = Vec::with_capacity(batch_size * seq_len);

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            for i in 0..seq_len {
                input_ids_flat.push(ids.get(i).copied().unwrap_or(0) as i64);
                attention_mask_flat.push(mask.get(i).copied().unwrap_or(0) as i64);
                token_type_ids_flat.push(type_ids.get(i).copied().unwrap_or(0) as i64);
            }
        }

        // Build ort Tensors
        let input_ids_tensor = Tensor::from_array(([batch_size, seq_len], input_ids_flat))
            .map_err(|e| CortexError::Embedding(format!("Tensor error: {e}")))?;
        let attention_mask_tensor =
            Tensor::from_array(([batch_size, seq_len], attention_mask_flat.clone()))
                .map_err(|e| CortexError::Embedding(format!("Tensor error: {e}")))?;
        let token_type_ids_tensor =
            Tensor::from_array(([batch_size, seq_len], token_type_ids_flat))
                .map_err(|e| CortexError::Embedding(format!("Tensor error: {e}")))?;

        // 3. Run inference
        let mut session = self.session.lock()
            .map_err(|e| CortexError::Embedding(format!("Session lock error: {e}")))?;
        let outputs = session
            .run(ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => token_type_ids_tensor,
            ])
            .map_err(|e| CortexError::Embedding(format!("Inference failed: {e}")))?;

        // 4. Extract output tensor [batch_size, seq_len, hidden_dim]
        let output_value = &outputs[0];
        let (shape, raw_data): (&ort::tensor::Shape, &[f32]) = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| CortexError::Embedding(format!("Output extraction error: {e}")))?;

        let hidden_dim = shape[2] as usize;

        // Build ndarray for mean pooling
        let output_3d = ndarray::Array3::from_shape_vec(
            (batch_size, seq_len, hidden_dim),
            raw_data.to_vec(),
        )
        .map_err(|e| CortexError::Embedding(format!("Reshape error: {e}")))?;

        let attention_mask_2d =
            Array2::from_shape_vec((batch_size, seq_len), attention_mask_flat)
                .map_err(|e| CortexError::Embedding(format!("Mask shape error: {e}")))?;

        // 5. Mean pool + L2 normalize
        let results = mean_pool_and_normalize(&output_3d, &attention_mask_2d);

        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}
