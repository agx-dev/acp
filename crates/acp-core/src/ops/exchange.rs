use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::*;
use crate::AcpError;

/// Trait for agent exchange operations (Conformance: Core for export/import).
#[async_trait]
pub trait Exchange: Send + Sync {
    /// Export the full agent state.
    async fn export_agent(&self) -> Result<AgentBundle, AcpError>;

    /// Import an agent state.
    async fn import_agent(&self, bundle: AgentBundle) -> Result<(), AcpError>;
}

/// Complete agent state bundle for exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBundle {
    pub identity: AgentIdentity,
    pub episodes: Vec<Episode>,
    pub semantic_entries: Vec<SemanticEntry>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub skills: Vec<SkillObject>,
    pub snapshots: Vec<SnapshotInfo>,
}

/// Access level granted to a share recipient (`acp.exchange.share`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// The recipient may read the shared memory but not write back (default).
    #[default]
    ReadOnly,
    /// The recipient may read and contribute back to the shared memory.
    ReadWrite,
}

/// Optional content filter applied when sharing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShareFilter {
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Number of entries included in a share, per layer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ShareCounts {
    pub episodes: u64,
    pub semantic_entries: u64,
    pub nodes: u64,
    pub edges: u64,
    pub skills: u64,
}

/// A selective, layer-filtered memory share destined for another agent.
///
/// Reference implementation: `share` produces a filtered [`AgentBundle`] plus a
/// manifest describing the recipient, granted access level and applied filter.
/// Delivery/transport to the recipient is out of scope for the local server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareManifest {
    pub share_id: String,
    pub recipient: String,
    pub access_level: AccessLevel,
    /// Layer names included in the share (e.g. `["semantic", "procedural"]`).
    pub layers: Vec<String>,
    #[serde(default)]
    pub filter_tags: Vec<String>,
    pub counts: ShareCounts,
    pub bundle: AgentBundle,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
