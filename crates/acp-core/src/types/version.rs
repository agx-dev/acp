use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::Layer;

/// Configuration for creating a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub reason: String,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub parent: Option<String>,
}

/// Information about a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub layers: Vec<Layer>,
    pub tags: Vec<String>,
    pub parent: Option<String>,
    pub stats: SnapshotStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub episodes_count: u64,
    pub semantic_count: u64,
    pub nodes_count: u64,
    pub edges_count: u64,
    pub skills_count: u64,
    pub size_bytes: u64,
}

/// Diff between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub from: String,
    pub to: String,
    pub added: DiffCounts,
    pub removed: DiffCounts,
    pub modified: DiffCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffCounts {
    pub episodes: u64,
    pub semantic_entries: u64,
    pub nodes: u64,
    pub edges: u64,
    pub skills: u64,
}
