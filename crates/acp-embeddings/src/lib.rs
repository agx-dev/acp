//! # ACP Embeddings — Multi-Provider Embedding Abstraction
//!
//! Provides a trait-based abstraction for generating text embeddings,
//! with a mock provider for testing, an OpenAI provider (behind feature flag),
//! an LRU cache, and vector math utilities.

pub mod cache;
pub mod normalize;
pub mod provider;
pub mod providers;

pub use cache::EmbeddingCache;
pub use normalize::{cosine_similarity, euclidean_distance, normalize};
pub use provider::EmbeddingProvider;
pub use providers::mock::MockEmbeddings;

#[cfg(feature = "openai")]
pub use providers::openai::{OpenAIConfig, OpenAIEmbeddings, OpenAIModel};

use acp_core::AcpError;
use async_trait::async_trait;

/// Embedding provider wrapped with a transparent LRU cache.
pub struct CachedProvider {
    inner: Box<dyn EmbeddingProvider>,
    cache: EmbeddingCache,
}

impl CachedProvider {
    pub fn new(inner: Box<dyn EmbeddingProvider>, cache_size: usize) -> Self {
        Self {
            inner,
            cache: EmbeddingCache::new(cache_size),
        }
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }
}

#[async_trait]
impl EmbeddingProvider for CachedProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AcpError> {
        if let Some(cached) = self.cache.get(self.inner.model_id(), text) {
            return Ok(cached);
        }

        let embedding = self.inner.embed(text).await?;
        self.cache
            .put(self.inner.model_id(), text, embedding.clone());
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, AcpError> {
        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_texts = Vec::new();
        let mut uncached_indices = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            if let Some(cached) = self.cache.get(self.inner.model_id(), text) {
                results.push(cached);
            } else {
                results.push(Vec::new());
                uncached_texts.push(*text);
                uncached_indices.push(i);
            }
        }

        if !uncached_texts.is_empty() {
            let embeddings = self.inner.embed_batch(&uncached_texts).await?;
            for (idx, embedding) in uncached_indices.iter().zip(embeddings) {
                self.cache
                    .put(self.inner.model_id(), texts[*idx], embedding.clone());
                results[*idx] = embedding;
            }
        }

        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    fn model_id(&self) -> &str {
        self.inner.model_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embeddings() {
        let provider = MockEmbeddings::new(384);

        let emb = provider.embed("hello world").await.unwrap();
        assert_eq!(emb.len(), 384);

        // Deterministic: same text = same embedding
        let emb2 = provider.embed("hello world").await.unwrap();
        assert_eq!(emb, emb2);

        // Different text = different embedding
        let emb3 = provider.embed("goodbye world").await.unwrap();
        assert_ne!(emb, emb3);
    }

    #[tokio::test]
    async fn test_mock_batch() {
        let provider = MockEmbeddings::new(384);
        let results = provider
            .embed_batch(&["hello", "world", "test"])
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
        for emb in &results {
            assert_eq!(emb.len(), 384);
        }
    }

    #[tokio::test]
    async fn test_cached_provider() {
        let mock = MockEmbeddings::new(384);
        let cached = CachedProvider::new(Box::new(mock), 100);

        let emb1 = cached.embed("test").await.unwrap();
        assert_eq!(cached.cache_len(), 1);

        let emb2 = cached.embed("test").await.unwrap();
        assert_eq!(emb1, emb2);
        assert_eq!(cached.cache_len(), 1); // no new entry
    }

    #[tokio::test]
    async fn test_cached_batch() {
        let mock = MockEmbeddings::new(384);
        let cached = CachedProvider::new(Box::new(mock), 100);

        cached.embed("alpha").await.unwrap();

        let results = cached.embed_batch(&["alpha", "beta"]).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].len(), 384);
        assert_eq!(results[1].len(), 384);
        assert_eq!(cached.cache_len(), 2);
    }

    #[test]
    fn test_normalize() {
        let v = vec![3.0, 4.0];
        let n = normalize(&v);
        let norm: f32 = n.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let v = vec![0.0, 0.0, 0.0];
        let n = normalize(&v);
        assert_eq!(n, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = normalize(&vec![1.0, 0.0]);
        let b = normalize(&vec![1.0, 0.0]);
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = normalize(&vec![1.0, 0.0]);
        let b = normalize(&vec![0.0, 1.0]);
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = EmbeddingCache::new(2);
        cache.put("model", "a", vec![1.0]);
        cache.put("model", "b", vec![2.0]);
        cache.put("model", "c", vec![3.0]); // evicts "a"

        assert!(cache.get("model", "a").is_none());
        assert!(cache.get("model", "b").is_some());
        assert!(cache.get("model", "c").is_some());
    }

    #[test]
    fn test_model_id_and_dimensions() {
        let mock = MockEmbeddings::new(768);
        assert_eq!(mock.model_id(), "mock-embeddings");
        assert_eq!(mock.dimensions(), 768);
        assert_eq!(mock.max_batch_size(), 100);
        assert_eq!(mock.max_tokens(), 8192);
    }
}
