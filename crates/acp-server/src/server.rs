use std::path::PathBuf;

use acp_core::*;
use acp_embeddings::{CachedProvider, EmbeddingProvider, MockEmbeddings};
use acp_store::SqliteStore;

/// ACP Server — orchestrates store and embeddings.
///
/// The `SqliteStore` handles both memory storage AND graph persistence
/// (via `ContextGraphStore` trait), so no separate graph field is needed.
pub struct AcpServer {
    pub store: SqliteStore,
    pub(crate) embeddings: Box<dyn EmbeddingProvider>,
    pub(crate) started_at: std::time::Instant,
}

/// Configuration for creating an AcpServer with a specific embedding provider.
pub struct ServerConfig {
    pub storage_path: PathBuf,
    pub embedding_provider: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
}

impl AcpServer {
    pub fn with_config(config: ServerConfig) -> Result<Self, AcpError> {
        std::fs::create_dir_all(&config.storage_path)
            .map_err(|e| AcpError::Internal(format!("Cannot create storage dir: {}", e)))?;

        let db_path = config.storage_path.join("acp.db");
        let store = SqliteStore::open(&db_path).map_err(|e| AcpError::Internal(e.to_string()))?;

        let embeddings = create_embedding_provider(
            &config.embedding_provider,
            config.openai_api_key.as_deref(),
            &config.openai_model,
        )?;

        Ok(Self { store, embeddings, started_at: std::time::Instant::now() })
    }

    pub fn in_memory() -> Result<Self, AcpError> {
        let store = SqliteStore::in_memory().map_err(|e| AcpError::Internal(e.to_string()))?;
        let mock = MockEmbeddings::new(384);
        let embeddings: Box<dyn EmbeddingProvider> =
            Box::new(CachedProvider::new(Box::new(mock), 1_000));

        Ok(Self { store, embeddings, started_at: std::time::Instant::now() })
    }

    /// Create an in-memory server with a custom embedding provider (for testing).
    pub fn in_memory_with_provider(embeddings: Box<dyn EmbeddingProvider>) -> Result<Self, AcpError> {
        let store = SqliteStore::in_memory().map_err(|e| AcpError::Internal(e.to_string()))?;
        Ok(Self { store, embeddings, started_at: std::time::Instant::now() })
    }
}

fn create_embedding_provider(
    provider_name: &str,
    #[allow(unused_variables)] api_key: Option<&str>,
    #[allow(unused_variables)] model: &str,
) -> Result<Box<dyn EmbeddingProvider>, AcpError> {
    match provider_name {
        #[cfg(feature = "openai")]
        "openai" => {
            let key = api_key
                .map(String::from)
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    AcpError::Internal(
                        "OPENAI_API_KEY required when using openai embedding provider".into(),
                    )
                })?;

            let openai_model = match model {
                "text-embedding-3-large" => acp_embeddings::OpenAIModel::TextEmbedding3Large,
                "text-embedding-ada-002" => acp_embeddings::OpenAIModel::Ada002,
                _ => acp_embeddings::OpenAIModel::TextEmbedding3Small,
            };

            let config = acp_embeddings::OpenAIConfig {
                api_key: key,
                model: openai_model,
                ..Default::default()
            };

            let provider = acp_embeddings::OpenAIEmbeddings::new(config)?;
            Ok(Box::new(CachedProvider::new(Box::new(provider), 10_000)))
        }
        #[cfg(not(feature = "openai"))]
        "openai" => Err(AcpError::Internal(
            "openai provider not compiled in; rebuild with --features openai".into(),
        )),
        _ => {
            tracing::info!("Using mock embedding provider (384 dimensions)");
            let mock = MockEmbeddings::new(384);
            Ok(Box::new(CachedProvider::new(Box::new(mock), 10_000)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_is_selected_and_works() {
        let provider = create_embedding_provider("mock", None, "text-embedding-3-small")
            .expect("mock provider must always be available");
        assert_eq!(provider.model_id(), "mock-embeddings");
        assert_eq!(provider.dimensions(), 384);

        // The selected provider is fully functional (deterministic mock).
        let emb = provider.embed("hello").await.unwrap();
        assert_eq!(emb.len(), 384);
    }

    #[test]
    fn unknown_provider_falls_back_to_mock() {
        let provider = create_embedding_provider("something-else", None, "ignored")
            .expect("unknown provider name must fall back to mock");
        assert_eq!(provider.model_id(), "mock-embeddings");
    }

    #[test]
    #[cfg(not(feature = "openai"))]
    fn openai_without_feature_returns_clear_error() {
        let result =
            create_embedding_provider("openai", Some("sk-dummy"), "text-embedding-3-small");
        match result {
            Err(AcpError::Internal(msg)) => {
                assert!(
                    msg.contains("openai provider not compiled in"),
                    "unexpected error message: {msg}"
                );
                assert!(msg.contains("--features openai"), "message should tell how to fix it");
            }
            Err(other) => panic!("expected AcpError::Internal, got {other:?}"),
            Ok(_) => panic!("openai must fail when the feature is not compiled in"),
        }
    }

    #[test]
    #[cfg(feature = "openai")]
    fn openai_with_feature_and_dummy_key_constructs_without_network() {
        // Construction must succeed with a dummy key; no API call is made here.
        let provider =
            create_embedding_provider("openai", Some("sk-dummy"), "text-embedding-3-small")
                .expect("openai provider should construct with a dummy key");
        assert_eq!(provider.model_id(), "text-embedding-3-small");
        assert_eq!(provider.dimensions(), 1536);
    }

    #[test]
    #[cfg(feature = "openai")]
    fn openai_with_feature_but_no_key_errors() {
        // Ensure no ambient env var leaks a key into this test.
        std::env::remove_var("OPENAI_API_KEY");
        let result = create_embedding_provider("openai", None, "text-embedding-3-small");
        assert!(
            matches!(result, Err(AcpError::Internal(_))),
            "openai without a key must fail with Internal error"
        );
    }
}
