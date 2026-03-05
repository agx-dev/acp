use acp_core::AcpError;
use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::normalize::normalize;
use crate::provider::EmbeddingProvider;

/// Deterministic mock provider for testing.
pub struct MockEmbeddings {
    dimensions: usize,
}

impl MockEmbeddings {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AcpError> {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();

        let embedding: Vec<f32> = (0..self.dimensions)
            .map(|i| {
                let byte = hash[i % 32];
                (byte as f32 / 255.0) * 2.0 - 1.0
            })
            .collect();

        Ok(normalize(&embedding))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, AcpError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_id(&self) -> &str {
        "mock-embeddings"
    }
}
