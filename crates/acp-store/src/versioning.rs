use async_trait::async_trait;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use acp_core::types::graph::{Edge, Node};
use acp_core::types::version::*;
use acp_core::{AcpError, EntryId, VersionManager};

use crate::graph::{insert_edge, insert_node};
use crate::store::SqliteStore;

/// Internal snapshot data stored as JSON BLOB.
///
/// Nodes and edges are captured in full (not just ids) because the graph has no
/// soft-delete: restoring a snapshot must be able to re-insert nodes/edges that
/// were deleted after the snapshot was taken.
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotData {
    episode_ids: Vec<String>,
    semantic_ids: Vec<String>,
    skill_ids: Vec<String>,
    /// Full graph node state at snapshot time. `#[serde(default)]` keeps
    /// older-shaped snapshot blobs (without this field) deserializable.
    #[serde(default)]
    nodes: Vec<Node>,
    /// Full graph edge state at snapshot time.
    #[serde(default)]
    edges: Vec<Edge>,
}

#[async_trait]
impl VersionManager for SqliteStore {
    async fn snapshot(&self, config: SnapshotConfig) -> Result<SnapshotInfo, AcpError> {
        let conn = self.conn();

        // Get next version number
        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM snapshots",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        // Collect current state IDs
        let episode_ids = collect_ids(&conn, "SELECT id FROM episodes WHERE deleted_at IS NULL")?;
        let semantic_ids = collect_ids(
            &conn,
            "SELECT id FROM semantic_entries WHERE deleted_at IS NULL",
        )?;
        let skill_ids = collect_ids(&conn, "SELECT id FROM skills")?;

        // Capture the full graph state (nodes + edges) from the in-memory engine.
        // engine_export() locks graph_read internally, not the SQLite mutex, so it
        // is safe to call while holding `conn`.
        let exported = self.engine_export();

        let data = SnapshotData {
            episode_ids,
            semantic_ids,
            skill_ids,
            nodes: exported.nodes,
            edges: exported.edges,
        };

        let json_bytes =
            serde_json::to_vec(&data).map_err(|e| AcpError::Internal(e.to_string()))?;
        let size_bytes = json_bytes.len() as i64;

        // Simple hash of the data
        let hash = format!("{:x}", md5_hash(&json_bytes));

        let id = EntryId::new("snap");

        conn.execute(
            "INSERT INTO snapshots (id, version, hash, data, reason, size_bytes, compressed_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
            params![id.0, version, hash, json_bytes, config.reason, size_bytes, size_bytes],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        let stats = SnapshotStats {
            episodes_count: data.episode_ids.len() as u64,
            semantic_count: data.semantic_ids.len() as u64,
            nodes_count: data.nodes.len() as u64,
            edges_count: data.edges.len() as u64,
            skills_count: data.skill_ids.len() as u64,
            size_bytes: size_bytes as u64,
        };

        Ok(SnapshotInfo {
            id: id.0,
            reason: config.reason,
            created_at: chrono::Utc::now(),
            layers: config.layers,
            tags: config.tags,
            parent: config.parent,
            stats,
        })
    }

    async fn restore(&self, version: &str) -> Result<(), AcpError> {
        let conn = self.conn();

        // Find snapshot by id or version number
        let data_blob: Vec<u8> = conn
            .query_row(
                "SELECT data FROM snapshots WHERE id = ?1 OR CAST(version AS TEXT) = ?1",
                params![version],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AcpError::VersionNotFound(version.to_string()),
                other => AcpError::Internal(other.to_string()),
            })?;

        let data: SnapshotData =
            serde_json::from_slice(&data_blob).map_err(|e| AcpError::Internal(e.to_string()))?;

        // Restore episodes: soft-delete those not in snapshot, re-activate those in snapshot
        if !data.episode_ids.is_empty() {
            let placeholders = data
                .episode_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "UPDATE episodes SET deleted_at = datetime('now') WHERE deleted_at IS NULL AND id NOT IN ({})",
                placeholders
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let params: Vec<&dyn rusqlite::types::ToSql> = data
                .episode_ids
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;

            // Re-activate entries that were deleted after the snapshot
            let sql = format!(
                "UPDATE episodes SET deleted_at = NULL WHERE deleted_at IS NOT NULL AND id IN ({})",
                placeholders
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        } else {
            conn.execute(
                "UPDATE episodes SET deleted_at = datetime('now') WHERE deleted_at IS NULL",
                [],
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        }

        // Restore semantic entries: soft-delete those not in snapshot, re-activate those in snapshot
        if !data.semantic_ids.is_empty() {
            let placeholders = data
                .semantic_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "UPDATE semantic_entries SET deleted_at = datetime('now') WHERE deleted_at IS NULL AND id NOT IN ({})",
                placeholders
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let params: Vec<&dyn rusqlite::types::ToSql> = data
                .semantic_ids
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;

            // Re-activate entries that were deleted after the snapshot
            let sql = format!(
                "UPDATE semantic_entries SET deleted_at = NULL WHERE deleted_at IS NOT NULL AND id IN ({})",
                placeholders
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        } else {
            conn.execute(
                "UPDATE semantic_entries SET deleted_at = datetime('now') WHERE deleted_at IS NULL",
                [],
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        }

        // Skills don't have soft-delete, so delete those not in snapshot
        if !data.skill_ids.is_empty() {
            let placeholders = data
                .skill_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("DELETE FROM skills WHERE id NOT IN ({})", placeholders);
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let params: Vec<&dyn rusqlite::types::ToSql> = data
                .skill_ids
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        } else {
            conn.execute("DELETE FROM skills", [])
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        }

        // Restore the graph: wipe current nodes/edges and re-insert the snapshot's.
        // Nodes/edges have no soft-delete, so we replace the persisted state wholesale.
        conn.execute("DELETE FROM edges", [])
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM nodes", [])
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        for node in &data.nodes {
            insert_node(&conn, node)?;
        }
        for edge in &data.edges {
            insert_edge(&conn, edge)?;
        }

        // Drop the SQLite guard before rebuilding the engine: load_graph() locks
        // the SQLite mutex itself, so holding `conn` here would deadlock.
        drop(conn);

        // Rebuild the in-memory engine from scratch so it matches SQLite exactly.
        self.reset_graph_engine();
        self.load_graph()?;

        Ok(())
    }

    async fn diff(&self, from: &str, to: &str) -> Result<VersionDiff, AcpError> {
        let conn = self.conn();

        let load_snapshot = |version: &str| -> Result<SnapshotData, AcpError> {
            let blob: Vec<u8> = conn
                .query_row(
                    "SELECT data FROM snapshots WHERE id = ?1 OR CAST(version AS TEXT) = ?1",
                    params![version],
                    |row| row.get(0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AcpError::VersionNotFound(version.to_string())
                    }
                    other => AcpError::Internal(other.to_string()),
                })?;
            serde_json::from_slice(&blob).map_err(|e| AcpError::Internal(e.to_string()))
        };

        let from_data = load_snapshot(from)?;
        let to_data = load_snapshot(to)?;

        let diff_count = |from_ids: &[String], to_ids: &[String]| -> (u64, u64) {
            use std::collections::HashSet;
            let from_set: HashSet<&str> = from_ids.iter().map(|s| s.as_str()).collect();
            let to_set: HashSet<&str> = to_ids.iter().map(|s| s.as_str()).collect();
            let added = to_set.difference(&from_set).count() as u64;
            let removed = from_set.difference(&to_set).count() as u64;
            (added, removed)
        };

        let (ep_added, ep_removed) = diff_count(&from_data.episode_ids, &to_data.episode_ids);
        let (sem_added, sem_removed) = diff_count(&from_data.semantic_ids, &to_data.semantic_ids);
        let (sk_added, sk_removed) = diff_count(&from_data.skill_ids, &to_data.skill_ids);

        let from_node_ids: Vec<String> = from_data.nodes.iter().map(|n| n.id.0.clone()).collect();
        let to_node_ids: Vec<String> = to_data.nodes.iter().map(|n| n.id.0.clone()).collect();
        let (node_added, node_removed) = diff_count(&from_node_ids, &to_node_ids);

        let from_edge_ids: Vec<String> = from_data.edges.iter().map(|e| e.id.0.clone()).collect();
        let to_edge_ids: Vec<String> = to_data.edges.iter().map(|e| e.id.0.clone()).collect();
        let (edge_added, edge_removed) = diff_count(&from_edge_ids, &to_edge_ids);

        Ok(VersionDiff {
            from: from.to_string(),
            to: to.to_string(),
            added: DiffCounts {
                episodes: ep_added,
                semantic_entries: sem_added,
                nodes: node_added,
                edges: edge_added,
                skills: sk_added,
            },
            removed: DiffCounts {
                episodes: ep_removed,
                semantic_entries: sem_removed,
                nodes: node_removed,
                edges: edge_removed,
                skills: sk_removed,
            },
            modified: DiffCounts::default(),
        })
    }

    async fn list(&self) -> Result<Vec<SnapshotInfo>, AcpError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, version, data, reason, size_bytes, created_at
                 FROM snapshots ORDER BY version DESC",
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let _version: i64 = row.get(1)?;
                let data_blob: Vec<u8> = row.get(2)?;
                let reason: Option<String> = row.get(3)?;
                let size_bytes: i64 = row.get(4)?;
                let created_at_str: String = row.get(5)?;

                let data: SnapshotData =
                    serde_json::from_slice(&data_blob).unwrap_or(SnapshotData {
                        episode_ids: vec![],
                        semantic_ids: vec![],
                        skill_ids: vec![],
                        nodes: vec![],
                        edges: vec![],
                    });

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                Ok(SnapshotInfo {
                    id,
                    reason: reason.unwrap_or_default(),
                    created_at,
                    layers: vec![],
                    tags: vec![],
                    parent: None,
                    stats: SnapshotStats {
                        episodes_count: data.episode_ids.len() as u64,
                        semantic_count: data.semantic_ids.len() as u64,
                        nodes_count: data.nodes.len() as u64,
                        edges_count: data.edges.len() as u64,
                        skills_count: data.skill_ids.len() as u64,
                        size_bytes: size_bytes as u64,
                    },
                })
            })
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }
}

impl SqliteStore {
    /// Resolve a snapshot reference (id or version number) to its snapshot id.
    fn resolve_snapshot_id(&self, version: &str) -> Result<String, AcpError> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id FROM snapshots WHERE id = ?1 OR CAST(version AS TEXT) = ?1",
            params![version],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AcpError::VersionNotFound(version.to_string()),
            other => AcpError::Internal(other.to_string()),
        })
    }

    /// Create a named branch pointing at a snapshot (`acp.version.branch`).
    ///
    /// A branch is a named pointer to a checkpoint of cognitive state. When
    /// `from_version` is omitted, a fresh snapshot of the current state is taken
    /// and used as the branch base.
    pub async fn branch(&self, config: BranchConfig) -> Result<BranchInfo, AcpError> {
        let (head_snapshot, base_version) = match &config.from_version {
            Some(v) => (self.resolve_snapshot_id(v)?, Some(v.clone())),
            None => {
                let info = self
                    .snapshot(SnapshotConfig {
                        reason: format!("branch base: {}", config.name),
                        layers: vec![],
                        tags: vec![format!("branch:{}", config.name)],
                        parent: None,
                    })
                    .await?;
                (info.id, None)
            }
        };

        let created_at = chrono::Utc::now();
        {
            let conn = self.conn();
            let res = conn.execute(
                "INSERT INTO branches (name, head_snapshot, base_version, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    config.name,
                    head_snapshot,
                    base_version,
                    created_at.to_rfc3339()
                ],
            );
            match res {
                Ok(_) => {}
                Err(rusqlite::Error::SqliteFailure(e, _))
                    if e.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    return Err(AcpError::InvalidParams(format!(
                        "branch already exists: {}",
                        config.name
                    )));
                }
                Err(e) => return Err(AcpError::Internal(e.to_string())),
            }
        }

        Ok(BranchInfo {
            name: config.name,
            head_snapshot,
            base_version,
            created_at,
        })
    }

    /// Merge a branch back into the main line (`acp.version.merge`).
    ///
    /// `prefer_branch` adopts the branch's checkpointed state; `prefer_main`
    /// keeps the current state. Either way a result snapshot is taken and the
    /// applied delta is reported.
    pub async fn merge_branch(
        &self,
        req: BranchMergeRequest,
    ) -> Result<BranchMergeReport, AcpError> {
        let head_snapshot: String = {
            let conn = self.conn();
            conn.query_row(
                "SELECT head_snapshot FROM branches WHERE name = ?1",
                params![req.branch],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AcpError::VersionNotFound(format!("branch:{}", req.branch))
                }
                other => AcpError::Internal(other.to_string()),
            })?
        };

        // Checkpoint the pre-merge state so we can report the applied delta.
        let pre = self
            .snapshot(SnapshotConfig {
                reason: format!("pre-merge: {}", req.branch),
                layers: vec![],
                tags: vec![],
                parent: None,
            })
            .await?;

        if req.strategy == MergeStrategy::PreferBranch {
            self.restore(&head_snapshot).await?;
        }
        // PreferMain keeps the current state (no-op).

        let result = self
            .snapshot(SnapshotConfig {
                reason: format!("merge: {}", req.branch),
                layers: vec![],
                tags: vec![],
                parent: Some(pre.id.clone()),
            })
            .await?;

        let diff = self.diff(&pre.id, &result.id).await?;
        let merged = DiffCounts {
            episodes: diff.added.episodes + diff.removed.episodes,
            semantic_entries: diff.added.semantic_entries + diff.removed.semantic_entries,
            nodes: 0,
            edges: 0,
            skills: diff.added.skills + diff.removed.skills,
        };

        Ok(BranchMergeReport {
            branch: req.branch,
            strategy: req.strategy,
            result_snapshot: result.id,
            merged,
        })
    }
}

fn collect_ids(conn: &rusqlite::Connection, sql: &str) -> Result<Vec<String>, AcpError> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| AcpError::Internal(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| AcpError::Internal(e.to_string()))?;
    rows.collect::<Result<Vec<String>, _>>()
        .map_err(|e| AcpError::Internal(e.to_string()))
}

/// Simple hash for snapshot integrity (not cryptographic).
fn md5_hash(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use acp_core::types::graph::{NodeType, Relation};
    use acp_core::types::semantic::SemanticSource;
    use acp_core::{ContextGraphStore, Confidence, Layer, MemoryStore, SemanticEntry, StoreEntry};

    fn test_node(id: &str, label: &str) -> Node {
        Node {
            id: EntryId(id.to_string()),
            node_type: NodeType::Task,
            label: label.to_string(),
            properties: Default::default(),
            embedding: None,
            episode_refs: vec![],
            semantic_refs: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn test_edge(id: &str, source: &str, target: &str) -> Edge {
        Edge {
            id: EntryId(id.to_string()),
            source: EntryId(source.to_string()),
            target: EntryId(target.to_string()),
            relation: Relation::LedTo,
            weight: 1.0,
            confidence: None,
            evidence: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn semantic(id: &str, content: &str) -> StoreEntry {
        StoreEntry::Semantic(SemanticEntry {
            id: EntryId(id.to_string()),
            content: content.to_string(),
            embedding: None,
            source: SemanticSource::Manual,
            confidence: Confidence::new(0.9).unwrap(),
            importance: 0.5,
            access_count: 0,
            last_accessed: None,
            tags: vec![],
            category: None,
            domain: None,
            protected: false,
            decay_rate: 0.01,
            provenance: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    #[tokio::test]
    async fn test_branch_from_current_state() {
        let store = SqliteStore::in_memory().unwrap();
        store.store(Layer::Semantic, semantic("sem-1", "A")).await.unwrap();

        let info = store
            .branch(BranchConfig {
                name: "experiment".into(),
                from_version: None,
            })
            .await
            .unwrap();
        assert_eq!(info.name, "experiment");
        assert!(info.head_snapshot.starts_with("snap-"));
    }

    #[tokio::test]
    async fn test_branch_duplicate_name_rejected() {
        let store = SqliteStore::in_memory().unwrap();
        store
            .branch(BranchConfig { name: "b1".into(), from_version: None })
            .await
            .unwrap();
        let err = store
            .branch(BranchConfig { name: "b1".into(), from_version: None })
            .await;
        assert!(matches!(err, Err(AcpError::InvalidParams(_))));
    }

    #[tokio::test]
    async fn test_branch_from_unknown_version_errors() {
        let store = SqliteStore::in_memory().unwrap();
        let err = store
            .branch(BranchConfig {
                name: "b".into(),
                from_version: Some("does-not-exist".into()),
            })
            .await;
        assert!(matches!(err, Err(AcpError::VersionNotFound(_))));
    }

    #[tokio::test]
    async fn test_merge_prefer_branch_adopts_branch_state() {
        let store = SqliteStore::in_memory().unwrap();
        // Branch base captures only "A".
        store.store(Layer::Semantic, semantic("sem-1", "A")).await.unwrap();
        let branch = store
            .branch(BranchConfig { name: "exp".into(), from_version: None })
            .await
            .unwrap();

        // Main line diverges by adding "B".
        store.store(Layer::Semantic, semantic("sem-2", "B")).await.unwrap();

        let report = store
            .merge_branch(BranchMergeRequest {
                branch: "exp".into(),
                strategy: MergeStrategy::PreferBranch,
            })
            .await
            .unwrap();

        assert_eq!(report.branch, "exp");
        // Adopting the branch removes "B" -> one semantic entry changed.
        assert_eq!(report.merged.semantic_entries, 1);
        assert!(report.result_snapshot.starts_with("snap-"));
        let _ = branch;
    }

    #[tokio::test]
    async fn test_merge_unknown_branch_errors() {
        let store = SqliteStore::in_memory().unwrap();
        let err = store
            .merge_branch(BranchMergeRequest {
                branch: "ghost".into(),
                strategy: MergeStrategy::PreferBranch,
            })
            .await;
        assert!(matches!(err, Err(AcpError::VersionNotFound(_))));
    }

    #[tokio::test]
    async fn test_snapshot_captures_graph_nodes_and_edges() {
        let store = SqliteStore::in_memory().unwrap();
        store.add_node(test_node("n1", "A")).await.unwrap();
        store.add_node(test_node("n2", "B")).await.unwrap();
        store.add_edge(test_edge("e1", "n1", "n2")).await.unwrap();

        let info = store
            .snapshot(SnapshotConfig {
                reason: "graph snap".into(),
                layers: vec![],
                tags: vec![],
                parent: None,
            })
            .await
            .unwrap();

        assert_eq!(info.stats.nodes_count, 2);
        assert_eq!(info.stats.edges_count, 1);
    }

    #[tokio::test]
    async fn test_restore_reverts_graph_and_rebuilds_engine() {
        let store = SqliteStore::in_memory().unwrap();
        store.add_node(test_node("n1", "A")).await.unwrap();
        store.add_node(test_node("n2", "B")).await.unwrap();
        store.add_edge(test_edge("e1", "n1", "n2")).await.unwrap();

        let snap = store
            .snapshot(SnapshotConfig {
                reason: "before".into(),
                layers: vec![],
                tags: vec![],
                parent: None,
            })
            .await
            .unwrap();

        // Diverge: add a third node.
        store.add_node(test_node("n3", "C")).await.unwrap();
        assert_eq!(store.graph_node_count(), 3);

        store.restore(&snap.id).await.unwrap();

        // In-memory engine reflects the restored state.
        assert_eq!(store.graph_node_count(), 2);
        assert_eq!(store.graph_edge_count(), 1);
        assert!(store.graph_read().get_node("n3").is_none());
        assert!(store.graph_read().get_node("n1").is_some());

        // SQLite matches the engine.
        let conn = store.conn();
        let node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(node_count, 2);
    }

    #[tokio::test]
    async fn test_diff_counts_graph_nodes_added() {
        let store = SqliteStore::in_memory().unwrap();
        store.add_node(test_node("n1", "A")).await.unwrap();
        store.add_node(test_node("n2", "B")).await.unwrap();
        store.add_edge(test_edge("e1", "n1", "n2")).await.unwrap();

        let from = store
            .snapshot(SnapshotConfig {
                reason: "from".into(),
                layers: vec![],
                tags: vec![],
                parent: None,
            })
            .await
            .unwrap();

        store.add_node(test_node("n3", "C")).await.unwrap();

        let to = store
            .snapshot(SnapshotConfig {
                reason: "to".into(),
                layers: vec![],
                tags: vec![],
                parent: None,
            })
            .await
            .unwrap();

        let diff = store.diff(&from.id, &to.id).await.unwrap();
        assert_eq!(diff.added.nodes, 1);
        assert_eq!(diff.removed.nodes, 0);
    }
}
