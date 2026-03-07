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
    #[allow(dead_code)]
    embeddings: Box<dyn EmbeddingProvider>,
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

        Ok(Self { store, embeddings })
    }

    pub fn in_memory() -> Result<Self, AcpError> {
        let store = SqliteStore::in_memory().map_err(|e| AcpError::Internal(e.to_string()))?;
        let mock = MockEmbeddings::new(384);
        let embeddings: Box<dyn EmbeddingProvider> =
            Box::new(CachedProvider::new(Box::new(mock), 1_000));

        Ok(Self { store, embeddings })
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
            "OpenAI embeddings require the 'openai' feature flag. \
             Rebuild with: cargo build --features openai"
                .into(),
        )),
        _ => {
            tracing::info!("Using mock embedding provider (384 dimensions)");
            let mock = MockEmbeddings::new(384);
            Ok(Box::new(CachedProvider::new(Box::new(mock), 10_000)))
        }
    }
}
