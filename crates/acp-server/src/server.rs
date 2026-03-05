use std::path::PathBuf;

use acp_core::*;
use acp_embeddings::{CachedProvider, EmbeddingProvider, MockEmbeddings};
use acp_graph::GraphStore;
use acp_store::SqliteStore;

/// ACP Server — orchestrates store, graph, and embeddings.
pub struct AcpServer {
    pub(crate) store: SqliteStore,
    pub(crate) graph: GraphStore,
    #[allow(dead_code)]
    pub(crate) embeddings: Box<dyn EmbeddingProvider>,
}

impl AcpServer {
    pub fn new(storage_path: PathBuf) -> Result<Self, AcpError> {
        std::fs::create_dir_all(&storage_path)
            .map_err(|e| AcpError::Internal(format!("Cannot create storage dir: {}", e)))?;

        let db_path = storage_path.join("acp.db");
        let store = SqliteStore::open(&db_path).map_err(|e| AcpError::Internal(e.to_string()))?;
        let graph = GraphStore::new();

        let mock = MockEmbeddings::new(384);
        let embeddings: Box<dyn EmbeddingProvider> =
            Box::new(CachedProvider::new(Box::new(mock), 10_000));

        Ok(Self {
            store,
            graph,
            embeddings,
        })
    }

    pub fn in_memory() -> Result<Self, AcpError> {
        let store = SqliteStore::in_memory().map_err(|e| AcpError::Internal(e.to_string()))?;
        let graph = GraphStore::new();
        let mock = MockEmbeddings::new(384);
        let embeddings: Box<dyn EmbeddingProvider> =
            Box::new(CachedProvider::new(Box::new(mock), 1_000));

        Ok(Self {
            store,
            graph,
            embeddings,
        })
    }
}
