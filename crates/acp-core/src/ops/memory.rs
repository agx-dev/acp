use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::*;
use crate::AcpError;

/// Core trait for memory operations (Conformance: Core).
///
/// Any ACP-compliant backend must implement this trait.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store an entry in the specified layer.
    async fn store(&self, layer: Layer, entry: StoreEntry) -> Result<EntryId, AcpError>;

    /// Recall entries matching a query.
    async fn recall(&self, query: RecallQuery) -> Result<RecallResult, AcpError>;

    /// Forget an entry using the specified strategy.
    async fn forget(&self, id: &EntryId, strategy: ForgetStrategy) -> Result<(), AcpError>;

    /// Prune entries according to retention policy.
    async fn prune(&self, policy: &RetentionPolicy) -> Result<PruneReport, AcpError>;

    /// Get memory statistics.
    async fn stats(&self, layers: &[Layer]) -> Result<MemoryStats, AcpError>;
}

/// Entry to store — union of possible entry types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StoreEntry {
    Episode(Episode),
    Semantic(SemanticEntry),
    Skill(SkillObject),
}

/// Query for recalling entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecallQuery {
    /// Text query for semantic search.
    pub text: Option<String>,
    /// Layers to search in.
    #[serde(default)]
    pub layers: Vec<Layer>,
    /// Maximum number of results.
    pub top_k: Option<usize>,
    /// Minimum confidence threshold.
    pub min_confidence: Option<f64>,
    /// Filter by tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Filter by category.
    pub category: Option<String>,
    /// Filter by domain.
    pub domain: Option<String>,
    /// Include embeddings in results.
    #[serde(default)]
    pub include_embeddings: bool,
}

/// Result of a recall operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecallResult {
    pub entries: Vec<RecallEntry>,
    pub total_count: u64,
    pub query_time_ms: u64,
}

/// A single recall result with relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallEntry {
    pub id: EntryId,
    pub layer: Layer,
    pub content: String,
    pub score: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Memory statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub episodes_count: u64,
    pub semantic_count: u64,
    pub skills_count: u64,
    pub total_size_bytes: u64,
    pub layer_stats: Vec<LayerStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    pub layer: Layer,
    pub entry_count: u64,
    pub size_bytes: u64,
}
