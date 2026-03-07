use std::path::PathBuf;

use acp_core::*;
use acp_embeddings::{CachedProvider, EmbeddingProvider, MockEmbeddings};
use acp_store::SqliteStore;

/// ACP Server — orchestrates store and embeddings.
///
/// The `SqliteStore` now handles both memory storage AND graph persistence
/// (via `ContextGraphStore` trait), so no separate graph field is needed.
pub struct AcpServer {
    pub(crate) store: SqliteStore,
    #[allow(dead_code)]
    pub(crate) embeddings: Box<dyn EmbeddingProvider>,
}

impl AcpServer {
    pub fn new(storage_path: PathBuf) -> Result<Self, AcpError> {
        std::fs::create_dir_all(&storage_path)
            .map_err(|e| AcpError::Internal(format!("Cannot create storage dir: {}", e)))?;

        let db_path = storage_path.join("acp.db");
        let store = SqliteStore::open(&db_path).map_err(|e| AcpError::Internal(e.to_string()))?;

        let mock = MockEmbeddings::new(384);
        let embeddings: Box<dyn EmbeddingProvider> =
            Box::new(CachedProvider::new(Box::new(mock), 10_000));

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
