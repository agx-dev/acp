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
