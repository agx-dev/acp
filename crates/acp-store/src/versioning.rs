use async_trait::async_trait;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use acp_core::types::version::*;
use acp_core::{AcpError, EntryId, VersionManager};

use crate::store::SqliteStore;

/// Internal snapshot data stored as JSON BLOB.
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotData {
    episode_ids: Vec<String>,
    semantic_ids: Vec<String>,
    skill_ids: Vec<String>,
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
        let semantic_ids =
            collect_ids(&conn, "SELECT id FROM semantic_entries WHERE deleted_at IS NULL")?;
        let skill_ids = collect_ids(&conn, "SELECT id FROM skills")?;

        let data = SnapshotData {
            episode_ids,
            semantic_ids,
            skill_ids,
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
            nodes_count: 0,
            edges_count: 0,
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
                rusqlite::Error::QueryReturnedNoRows => {
                    AcpError::Internal(format!("Snapshot not found: {}", version))
                }
                other => AcpError::Internal(other.to_string()),
            })?;

        let data: SnapshotData = serde_json::from_slice(&data_blob)
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        // Soft-delete entries not in the snapshot
        if !data.episode_ids.is_empty() {
            let placeholders = data.episode_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "UPDATE episodes SET deleted_at = datetime('now') WHERE deleted_at IS NULL AND id NOT IN ({})",
                placeholders
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| AcpError::Internal(e.to_string()))?;
            let params: Vec<&dyn rusqlite::types::ToSql> =
                data.episode_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        } else {
            conn.execute(
                "UPDATE episodes SET deleted_at = datetime('now') WHERE deleted_at IS NULL",
                [],
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        }

        if !data.semantic_ids.is_empty() {
            let placeholders = data.semantic_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "UPDATE semantic_entries SET deleted_at = datetime('now') WHERE deleted_at IS NULL AND id NOT IN ({})",
                placeholders
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| AcpError::Internal(e.to_string()))?;
            let params: Vec<&dyn rusqlite::types::ToSql> =
                data.semantic_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
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
            let placeholders = data.skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!("DELETE FROM skills WHERE id NOT IN ({})", placeholders);
            let mut stmt = conn.prepare(&sql).map_err(|e| AcpError::Internal(e.to_string()))?;
            let params: Vec<&dyn rusqlite::types::ToSql> =
                data.skill_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.execute(params.as_slice())
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        } else {
            conn.execute("DELETE FROM skills", [])
                .map_err(|e| AcpError::Internal(e.to_string()))?;
        }

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
                        AcpError::Internal(format!("Snapshot not found: {}", version))
                    }
                    other => AcpError::Internal(other.to_string()),
                })?;
            serde_json::from_slice(&blob).map_err(|e| AcpError::Internal(e.to_string()))
        };

        let from_data = load_snapshot(from)?;
        let to_data = load_snapshot(to)?;

        let diff_count =
            |from_ids: &[String], to_ids: &[String]| -> (u64, u64) {
                use std::collections::HashSet;
                let from_set: HashSet<&str> = from_ids.iter().map(|s| s.as_str()).collect();
                let to_set: HashSet<&str> = to_ids.iter().map(|s| s.as_str()).collect();
                let added = to_set.difference(&from_set).count() as u64;
                let removed = from_set.difference(&to_set).count() as u64;
                (added, removed)
            };

        let (ep_added, ep_removed) = diff_count(&from_data.episode_ids, &to_data.episode_ids);
        let (sem_added, sem_removed) =
            diff_count(&from_data.semantic_ids, &to_data.semantic_ids);
        let (sk_added, sk_removed) = diff_count(&from_data.skill_ids, &to_data.skill_ids);

        Ok(VersionDiff {
            from: from.to_string(),
            to: to.to_string(),
            added: DiffCounts {
                episodes: ep_added,
                semantic_entries: sem_added,
                nodes: 0,
                edges: 0,
                skills: sk_added,
            },
            removed: DiffCounts {
                episodes: ep_removed,
                semantic_entries: sem_removed,
                nodes: 0,
                edges: 0,
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
                        nodes_count: 0,
                        edges_count: 0,
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

fn collect_ids(conn: &rusqlite::Connection, sql: &str) -> Result<Vec<String>, AcpError> {
    let mut stmt = conn.prepare(sql).map_err(|e| AcpError::Internal(e.to_string()))?;
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
