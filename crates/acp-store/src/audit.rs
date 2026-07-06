//! # Audit Trail (spec §11.4)
//!
//! Append-only audit log of security-relevant operations
//! (`memory.store/recall/forget/consolidate`, `skill.invoke`,
//! `version.snapshot/restore`, `exchange.share`, …).
//!
//! Entries follow the `audit_entry` shape from the spec:
//! `id`, `timestamp`, `event`, `actor`, and a `details` object carrying
//! `method` / `params` / `result`.

use acp_core::{AcpError, EntryId};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::store::SqliteStore;

/// A single audit-trail entry, serializable to the spec's `acp-audit-v1` shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry {
    /// `audit-<uuid>` identifier.
    pub id: String,
    /// RFC3339 timestamp of the event.
    pub timestamp: String,
    /// Short event name, e.g. `memory.recall`.
    pub event: String,
    /// Actor responsible, e.g. `agent:local`.
    pub actor: String,
    /// Wire method that produced the event, e.g. `acp.memory.recall`.
    pub method: String,
    /// Structured details (`params`, `result`, …).
    pub details: serde_json::Value,
}

impl SqliteStore {
    /// Append an audit entry (append-only — never updated or deleted).
    pub fn append_audit(
        &self,
        event: &str,
        actor: &str,
        method: &str,
        details: serde_json::Value,
    ) -> Result<(), AcpError> {
        let id = EntryId::new("audit").0;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details_json =
            serde_json::to_string(&details).map_err(|e| AcpError::Internal(e.to_string()))?;

        let conn = self.conn();
        conn.execute(
            "INSERT INTO audit_log (id, timestamp, event, actor, method, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, timestamp, event, actor, method, details_json],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Query the most recent audit entries, newest first.
    ///
    /// `event_filter` restricts results to a single event name when set.
    pub fn query_audit(
        &self,
        limit: usize,
        event_filter: Option<&str>,
    ) -> Result<Vec<AuditEntry>, AcpError> {
        let conn = self.conn();
        let limit = limit as i64;

        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<AuditEntry> {
            let details_str: String = row.get(5)?;
            let details: serde_json::Value =
                serde_json::from_str(&details_str).unwrap_or(serde_json::Value::Null);
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                event: row.get(2)?,
                actor: row.get(3)?,
                method: row.get(4)?,
                details,
            })
        };

        let entries = if let Some(event) = event_filter {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, event, actor, method, details
                     FROM audit_log
                     WHERE event = ?1
                     ORDER BY timestamp DESC, rowid DESC
                     LIMIT ?2",
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let rows = stmt
                .query_map(params![event, limit], map_row)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AcpError::Internal(e.to_string()))?
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, event, actor, method, details
                     FROM audit_log
                     ORDER BY timestamp DESC, rowid DESC
                     LIMIT ?1",
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let rows = stmt
                .query_map(params![limit], map_row)
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AcpError::Internal(e.to_string()))?
        };

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_append_and_query_audit() {
        let store = SqliteStore::in_memory().unwrap();

        store
            .append_audit(
                "memory.store",
                "agent:local",
                "acp.memory.store",
                json!({ "params": { "layer": "semantic" }, "result": { "id": "sem-1" } }),
            )
            .unwrap();

        let entries = store.query_audit(10, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "memory.store");
        assert_eq!(entries[0].actor, "agent:local");
        assert_eq!(entries[0].method, "acp.memory.store");
        assert!(entries[0].id.starts_with("audit-"));
        assert_eq!(entries[0].details["result"]["id"], "sem-1");
    }

    #[test]
    fn test_audit_after_store_and_recall() {
        let store = SqliteStore::in_memory().unwrap();

        store
            .append_audit("memory.store", "agent:local", "acp.memory.store", json!({}))
            .unwrap();
        store
            .append_audit(
                "memory.recall",
                "agent:local",
                "acp.memory.recall",
                json!({ "result": { "entries_returned": 3 } }),
            )
            .unwrap();

        let entries = store.query_audit(10, None).unwrap();
        assert_eq!(entries.len(), 2);
        // Newest first
        assert_eq!(entries[0].event, "memory.recall");
        assert_eq!(entries[1].event, "memory.store");
    }

    #[test]
    fn test_audit_event_filter() {
        let store = SqliteStore::in_memory().unwrap();

        store
            .append_audit("memory.store", "agent:local", "acp.memory.store", json!({}))
            .unwrap();
        store
            .append_audit("memory.recall", "agent:local", "acp.memory.recall", json!({}))
            .unwrap();
        store
            .append_audit("memory.store", "agent:local", "acp.memory.store", json!({}))
            .unwrap();

        let stores = store.query_audit(10, Some("memory.store")).unwrap();
        assert_eq!(stores.len(), 2);
        assert!(stores.iter().all(|e| e.event == "memory.store"));

        let recalls = store.query_audit(10, Some("memory.recall")).unwrap();
        assert_eq!(recalls.len(), 1);
    }

    #[test]
    fn test_audit_forget_is_recorded() {
        let store = SqliteStore::in_memory().unwrap();

        store
            .append_audit(
                "memory.forget",
                "agent:local",
                "acp.memory.forget",
                json!({ "params": { "id": "sem-42", "strategy": "hard" } }),
            )
            .unwrap();

        let entries = store.query_audit(10, Some("memory.forget")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].details["params"]["id"], "sem-42");
    }

    #[test]
    fn test_audit_query_limit() {
        let store = SqliteStore::in_memory().unwrap();
        for _ in 0..5 {
            store
                .append_audit("skill.invoke", "agent:local", "acp.skill.invoke", json!({}))
                .unwrap();
        }
        let entries = store.query_audit(3, None).unwrap();
        assert_eq!(entries.len(), 3);
    }
}
