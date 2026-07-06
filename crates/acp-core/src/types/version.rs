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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiffCounts {
    pub episodes: u64,
    pub semantic_entries: u64,
    pub nodes: u64,
    pub edges: u64,
    pub skills: u64,
}

/// Configuration for creating a branch (`acp.version.branch`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchConfig {
    pub name: String,
    /// Snapshot id/version to branch from. When omitted, a fresh snapshot of
    /// the current state is taken and used as the branch base.
    #[serde(default)]
    pub from_version: Option<String>,
}

/// A named branch pointing at a snapshot of cognitive state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchInfo {
    pub name: String,
    /// The snapshot the branch was created from / currently points at.
    pub head_snapshot: String,
    pub base_version: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Strategy for merging a branch back into the main line (`acp.version.merge`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// The branch's state wins on conflict (default).
    #[default]
    PreferBranch,
    /// The current main-line state wins on conflict.
    PreferMain,
}

/// Request for `acp.version.merge`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMergeRequest {
    pub branch: String,
    #[serde(default)]
    pub strategy: MergeStrategy,
}

/// Outcome of merging a branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchMergeReport {
    pub branch: String,
    pub strategy: MergeStrategy,
    /// Snapshot taken of the merged result.
    pub result_snapshot: String,
    pub merged: DiffCounts,
}
