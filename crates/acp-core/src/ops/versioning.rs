use async_trait::async_trait;

use crate::types::*;
use crate::AcpError;

/// Trait for version management (Conformance: Standard).
#[async_trait]
pub trait VersionManager: Send + Sync {
    /// Create a snapshot of the current cognitive state.
    async fn snapshot(&self, config: SnapshotConfig) -> Result<SnapshotInfo, AcpError>;

    /// Restore to a previous snapshot.
    async fn restore(&self, version: &str) -> Result<(), AcpError>;

    /// Diff between two snapshots.
    async fn diff(&self, from: &str, to: &str) -> Result<VersionDiff, AcpError>;

    /// List all snapshots.
    async fn list(&self) -> Result<Vec<SnapshotInfo>, AcpError>;
}
