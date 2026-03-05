use crate::types::Layer;

/// All ACP-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("Layer not found: {0:?}")]
    LayerNotFound(Layer),

    #[error("Entry not found: {0}")]
    EntryNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Retention violation: {0}")]
    RetentionViolation(String),

    #[error("Embedding dimension mismatch: expected {expected}, got {got}")]
    EmbeddingMismatch { expected: usize, got: usize },

    #[error("Graph cycle detected")]
    GraphCycle,

    #[error("Consolidation failed: {0}")]
    ConsolidationFailed(String),

    #[error("Snapshot limit reached")]
    SnapshotLimit,

    #[error("Memory budget exceeded")]
    BudgetExceeded,

    #[error("Protected entry: {0}")]
    ProtectedEntry(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Missing dependency: {0}")]
    DependencyMissing(String),

    #[error("Incompatible model: {0}")]
    ModelIncompatible(String),

    #[error("Invalid confidence value: {0}")]
    InvalidConfidence(f64),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl AcpError {
    /// JSON-RPC error code for this error.
    pub fn code(&self) -> i32 {
        match self {
            Self::LayerNotFound(_) => -32001,
            Self::EntryNotFound(_) => -32002,
            Self::VersionNotFound(_) => -32003,
            Self::SkillNotFound(_) => -32004,
            Self::AccessDenied(_) => -32005,
            Self::RetentionViolation(_) => -32006,
            Self::EmbeddingMismatch { .. } => -32007,
            Self::GraphCycle => -32008,
            Self::ConsolidationFailed(_) => -32009,
            Self::SnapshotLimit => -32010,
            Self::BudgetExceeded => -32011,
            Self::ProtectedEntry(_) => -32012,
            Self::MergeConflict(_) => -32013,
            Self::DependencyMissing(_) => -32014,
            Self::ModelIncompatible(_) => -32015,
            Self::InvalidConfidence(_) => -32602,
            Self::Internal(_) => -32603,
            Self::Serialization(_) => -32603,
        }
    }

    /// Convert to a JSON-RPC error.
    pub fn to_jsonrpc(&self) -> crate::protocol::JsonRpcError {
        crate::protocol::JsonRpcError {
            code: self.code(),
            message: self.to_string(),
            data: None,
        }
    }
}
