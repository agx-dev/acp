use serde::{Deserialize, Serialize};

/// Retention policy for memory management.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub episodic: EpisodicRetention,
    pub semantic: SemanticRetention,
    pub graph: GraphRetention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicRetention {
    /// Max number of episodes to keep.
    pub max_episodes: Option<u64>,
    /// Max age in days before eviction.
    pub max_age_days: Option<u64>,
    /// Strategy when limits are reached.
    pub eviction: EvictionStrategy,
}

impl Default for EpisodicRetention {
    fn default() -> Self {
        Self {
            max_episodes: Some(10_000),
            max_age_days: Some(90),
            eviction: EvictionStrategy::Fifo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRetention {
    /// Max number of semantic entries.
    pub max_entries: Option<u64>,
    /// Min importance to keep (below = candidate for eviction).
    pub min_importance: Option<f64>,
    /// Enable time-based decay.
    #[serde(default = "default_true")]
    pub decay_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SemanticRetention {
    fn default() -> Self {
        Self {
            max_entries: Some(5_000),
            min_importance: Some(0.1),
            decay_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRetention {
    pub max_nodes: Option<u64>,
    pub max_edges: Option<u64>,
    /// Remove orphan nodes (no edges).
    #[serde(default = "default_true")]
    pub prune_orphans: bool,
}

impl Default for GraphRetention {
    fn default() -> Self {
        Self {
            max_nodes: Some(10_000),
            max_edges: Some(50_000),
            prune_orphans: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvictionStrategy {
    /// First in, first out.
    Fifo,
    /// Lowest importance first.
    Importance,
    /// Time-decayed importance.
    Decay,
}

/// Strategy for forgetting an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForgetStrategy {
    /// Hard delete — entry is permanently removed.
    Hard,
    /// Soft delete — entry is marked as deleted but retained.
    Soft,
    /// Redact — content is removed but metadata is retained.
    Redact,
}

/// Report from a prune operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PruneReport {
    pub episodes_pruned: u64,
    pub semantic_pruned: u64,
    pub nodes_pruned: u64,
    pub edges_pruned: u64,
}
