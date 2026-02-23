/// Mean pooling over token embeddings, masked by attention mask.
///
/// `embeddings` shape: [batch_size, seq_len, hidden_dim]
/// `attention_mask` shape: [batch_size, seq_len]
/// Returns: [batch_size, hidden_dim] after mean pooling + L2 normalization.
pub fn mean_pool_and_normalize(
    embeddings: &ndarray::Array3<f32>,
    attention_mask: &ndarray::Array2<i64>,
) -> Vec<Vec<f32>> {
    let batch_size = embeddings.shape()[0];
    let seq_len = embeddings.shape()[1];
    let hidden_dim = embeddings.shape()[2];

    let mut results = Vec::with_capacity(batch_size);

    for b in 0..batch_size {
        let mut pooled = vec![0.0f32; hidden_dim];
        let mut count = 0.0f32;

        for s in 0..seq_len {
            let mask_val = attention_mask[[b, s]] as f32;
            if mask_val > 0.0 {
                for d in 0..hidden_dim {
                    pooled[d] += embeddings[[b, s, d]] * mask_val;
                }
                count += mask_val;
            }
        }

        // Mean
        if count > 0.0 {
            for d in 0..hidden_dim {
                pooled[d] /= count;
            }
        }

        // L2 normalize
        let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for d in 0..hidden_dim {
                pooled[d] /= norm;
            }
        }

        results.push(pooled);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;
    use ndarray::Array2;

    #[test]
    fn test_mean_pool_simple() {
        // 1 batch, 2 tokens, 3 dims
        let embeddings = Array3::from_shape_vec(
            (1, 2, 3),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )
        .unwrap();
        let mask = Array2::from_shape_vec((1, 2), vec![1i64, 1]).unwrap();

        let result = mean_pool_and_normalize(&embeddings, &mask);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 3);

        // Mean should be [2.5, 3.5, 4.5], then L2-normalized
        let norm = (2.5f32 * 2.5 + 3.5 * 3.5 + 4.5 * 4.5).sqrt();
        assert!((result[0][0] - 2.5 / norm).abs() < 1e-5);
    }

    #[test]
    fn test_masked_token_excluded() {
        let embeddings = Array3::from_shape_vec(
            (1, 2, 3),
            vec![1.0, 2.0, 3.0, 99.0, 99.0, 99.0],
        )
        .unwrap();
        // Second token masked out
        let mask = Array2::from_shape_vec((1, 2), vec![1i64, 0]).unwrap();

        let result = mean_pool_and_normalize(&embeddings, &mask);
        // Should only use first token [1, 2, 3], L2-normalized
        let norm = (1.0f32 + 4.0 + 9.0).sqrt();
        assert!((result[0][0] - 1.0 / norm).abs() < 1e-5);
    }
}
