use acp_core::AcpError;
use async_trait::async_trait;

/// Trait that all embedding providers must implement.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AcpError>;

    /// Generate embeddings for a batch of texts.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, AcpError>;

    /// Number of dimensions in the output vector.
    fn dimensions(&self) -> usize;

    /// Model identifier string.
    fn model_id(&self) -> &str;

    /// Maximum batch size.
    fn max_batch_size(&self) -> usize {
        100
    }

    /// Maximum tokens per text.
    fn max_tokens(&self) -> usize {
        8192
    }
}
