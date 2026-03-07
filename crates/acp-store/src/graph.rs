use async_trait::async_trait;
use rusqlite::params;

use acp_core::types::graph::*;
use acp_core::{AcpError, ContextGraphStore, EdgeId, EntryId, NodeId};
// Note: NodeId and EdgeId are type aliases for EntryId
use acp_graph::SerializedGraph;

use crate::store::SqliteStore;

impl SqliteStore {
    /// Load all nodes and edges from SQLite into the in-memory GraphEngine.
    pub(crate) fn load_graph(&self) -> Result<(), AcpError> {
        let conn = self.conn();
        let mut engine = self.graph_write();

        // Load nodes
        let mut stmt = conn
            .prepare(
                "SELECT id, node_type, label, properties, embedding,
                        episode_refs, semantic_refs, created_at, updated_at
                 FROM nodes",
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let nodes = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let node_type_str: String = row.get(1)?;
                let label: String = row.get(2)?;
                let properties_json: String = row.get(3)?;
                let embedding_blob: Option<Vec<u8>> = row.get(4)?;
                let episode_refs_json: String = row.get(5)?;
                let semantic_refs_json: String = row.get(6)?;
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;

                Ok((
                    id,
                    node_type_str,
                    label,
                    properties_json,
                    embedding_blob,
                    episode_refs_json,
                    semantic_refs_json,
                    created_at_str,
                    updated_at_str,
                ))
            })
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        for row in nodes {
            let (
                id,
                node_type_str,
                label,
                properties_json,
                embedding_blob,
                episode_refs_json,
                semantic_refs_json,
                created_at_str,
                updated_at_str,
            ) = row.map_err(|e| AcpError::Internal(e.to_string()))?;

            let node_type: NodeType =
                serde_json::from_value(serde_json::Value::String(node_type_str))
                    .map_err(|e| AcpError::Internal(format!("Invalid node_type: {}", e)))?;

            let properties = serde_json::from_str(&properties_json).unwrap_or_default();

            let embedding = embedding_blob.map(|blob| {
                blob.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect()
            });

            let episode_refs: Vec<String> =
                serde_json::from_str(&episode_refs_json).unwrap_or_default();
            let semantic_refs: Vec<String> =
                serde_json::from_str(&semantic_refs_json).unwrap_or_default();

            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let node = Node {
                id: EntryId(id),
                node_type,
                label,
                properties,
                embedding,
                episode_refs: episode_refs.into_iter().map(EntryId).collect(),
                semantic_refs: semantic_refs.into_iter().map(EntryId).collect(),
                created_at,
                updated_at,
            };

            // Ignore errors for duplicate IDs (shouldn't happen with clean data)
            let _ = engine.add_node(node);
        }

        // Load edges
        let mut stmt = conn
            .prepare(
                "SELECT id, source, target, relation, weight, confidence, evidence, created_at
                 FROM edges",
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let edges = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let source: String = row.get(1)?;
                let target: String = row.get(2)?;
                let relation_str: String = row.get(3)?;
                let weight: f64 = row.get(4)?;
                let confidence: Option<f64> = row.get(5)?;
                let evidence: Option<String> = row.get(6)?;
                let created_at_str: String = row.get(7)?;

                Ok((
                    id,
                    source,
                    target,
                    relation_str,
                    weight,
                    confidence,
                    evidence,
                    created_at_str,
                ))
            })
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        for row in edges {
            let (id, source, target, relation_str, weight, confidence, evidence, created_at_str) =
                row.map_err(|e| AcpError::Internal(e.to_string()))?;

            let relation: Relation =
                serde_json::from_value(serde_json::Value::String(relation_str))
                    .map_err(|e| AcpError::Internal(format!("Invalid relation: {}", e)))?;

            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let edge = Edge {
                id: EntryId(id),
                source: EntryId(source),
                target: EntryId(target),
                relation,
                weight,
                confidence,
                evidence: evidence.map(EntryId),
                created_at,
            };

            let _ = engine.add_edge(edge);
        }

        Ok(())
    }

    /// Persist a node to SQLite.
    fn persist_node(&self, node: &Node) -> Result<(), AcpError> {
        let conn = self.conn();

        let embedding_blob: Option<Vec<u8>> = node.embedding.as_ref().map(|emb| {
            emb.iter().flat_map(|f| f.to_le_bytes()).collect()
        });

        let node_type_sql = enum_to_sql(&node.node_type)?;

        conn.execute(
            "INSERT OR REPLACE INTO nodes (
                id, node_type, label, properties, embedding,
                episode_refs, semantic_refs, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                node.id.0,
                node_type_sql,
                node.label,
                serde_json::to_string(&node.properties)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                embedding_blob,
                serde_json::to_string(&node.episode_refs)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                serde_json::to_string(&node.semantic_refs)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                node.created_at.to_rfc3339(),
                node.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Persist an edge to SQLite.
    fn persist_edge(&self, edge: &Edge) -> Result<(), AcpError> {
        let conn = self.conn();

        let relation_sql = enum_to_sql(&edge.relation)?;

        conn.execute(
            "INSERT OR REPLACE INTO edges (
                id, source, target, relation, weight, confidence, evidence, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                edge.id.0,
                edge.source.0,
                edge.target.0,
                relation_sql,
                edge.weight,
                edge.confidence,
                edge.evidence.as_ref().map(|e| &e.0),
                edge.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Export the full graph (nodes + edges) for serialization.
    pub fn engine_export(&self) -> SerializedGraph {
        self.graph_read().export()
    }

    /// Get the number of nodes in the graph.
    pub fn graph_node_count(&self) -> usize {
        self.graph_read().node_count()
    }

    /// Get the number of edges in the graph.
    pub fn graph_edge_count(&self) -> usize {
        self.graph_read().edge_count()
    }
}

/// Convert a serde-serializable enum to its lowercase SQL string.
fn enum_to_sql<T: serde::Serialize>(val: &T) -> Result<String, AcpError> {
    let json = serde_json::to_value(val).map_err(|e| AcpError::Internal(e.to_string()))?;
    Ok(json.as_str().unwrap_or_default().to_string())
}

#[async_trait]
impl ContextGraphStore for SqliteStore {
    async fn add_node(&self, node: Node) -> Result<NodeId, AcpError> {
        // Persist first so we fail fast on SQL errors
        self.persist_node(&node)?;
        let mut engine = self.graph_write();
        engine.add_node(node)
    }

    async fn add_edge(&self, edge: Edge) -> Result<EdgeId, AcpError> {
        // Validate via engine first (cycle detection, missing nodes)
        let mut engine = self.graph_write();
        let id = engine.add_edge(edge.clone())?;
        drop(engine);
        // Persist to SQLite
        self.persist_edge(&edge)?;
        Ok(id)
    }

    async fn query(&self, pattern: GraphPattern) -> Result<Vec<Node>, AcpError> {
        let engine = self.graph_read();
        Ok(engine.query(&pattern))
    }

    async fn subgraph(
        &self,
        root: &NodeId,
        depth: u32,
        max_nodes: u32,
    ) -> Result<SubGraph, AcpError> {
        let engine = self.graph_read();
        engine.subgraph(&root.0, depth, max_nodes)
    }

    async fn traverse(
        &self,
        start: &NodeId,
        relation: Relation,
        depth: u32,
    ) -> Result<Vec<Node>, AcpError> {
        let engine = self.graph_read();
        let results = engine.traverse_bfs(&start.0, Some(relation), depth);
        Ok(results.into_iter().map(|(n, _)| n.clone()).collect())
    }

    async fn remove_node(&self, id: &NodeId) -> Result<(), AcpError> {
        let mut engine = self.graph_write();
        engine.remove_node(&id.0)?;
        drop(engine);
        // CASCADE in SQL handles edge cleanup
        let conn = self.conn();
        conn.execute("DELETE FROM nodes WHERE id = ?1", params![id.0])
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn remove_edge(&self, id: &EdgeId) -> Result<(), AcpError> {
        let mut engine = self.graph_write();
        engine.remove_edge(&id.0)?;
        drop(engine);
        let conn = self.conn();
        conn.execute("DELETE FROM edges WHERE id = ?1", params![id.0])
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn test_graph_persists_nodes_and_edges() {
        let store = SqliteStore::in_memory().unwrap();

        store.add_node(test_node("n1", "Task A")).await.unwrap();
        store.add_node(test_node("n2", "Task B")).await.unwrap();
        store
            .add_edge(test_edge("e1", "n1", "n2"))
            .await
            .unwrap();

        // Verify in-memory engine
        assert_eq!(store.graph_node_count(), 2);
        assert_eq!(store.graph_edge_count(), 1);

        // Verify SQLite persistence
        let conn = store.conn();
        let node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get(0))
            .unwrap();
        let edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(node_count, 2);
        assert_eq!(edge_count, 1);
    }

    #[tokio::test]
    async fn test_graph_remove_persists() {
        let store = SqliteStore::in_memory().unwrap();

        store.add_node(test_node("n1", "A")).await.unwrap();
        store.add_node(test_node("n2", "B")).await.unwrap();
        store
            .add_edge(test_edge("e1", "n1", "n2"))
            .await
            .unwrap();

        store
            .remove_node(&EntryId("n1".to_string()))
            .await
            .unwrap();

        assert_eq!(store.graph_node_count(), 1);

        let conn = store.conn();
        let node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(node_count, 1);

        // Edge should be gone via CASCADE
        let edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(edge_count, 0);
    }

    #[tokio::test]
    async fn test_graph_traverse_and_subgraph() {
        let store = SqliteStore::in_memory().unwrap();

        store.add_node(test_node("n1", "Root")).await.unwrap();
        store.add_node(test_node("n2", "Child")).await.unwrap();
        store
            .add_edge(test_edge("e1", "n1", "n2"))
            .await
            .unwrap();

        let nodes = store
            .traverse(&EntryId("n1".to_string()), Relation::LedTo, 2)
            .await
            .unwrap();
        assert_eq!(nodes.len(), 2);

        let sub = store
            .subgraph(&EntryId("n1".to_string()), 1, 10)
            .await
            .unwrap();
        assert_eq!(sub.nodes.len(), 2);
        assert_eq!(sub.edges.len(), 1);
    }

    #[tokio::test]
    async fn test_graph_query() {
        let store = SqliteStore::in_memory().unwrap();

        store.add_node(test_node("n1", "Auth task")).await.unwrap();
        store
            .add_node(Node {
                id: EntryId("n2".to_string()),
                node_type: NodeType::Tool,
                label: "JWT lib".to_string(),
                properties: Default::default(),
                embedding: None,
                episode_refs: vec![],
                semantic_refs: vec![],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        let results = store
            .query(GraphPattern {
                node_type: Some(NodeType::Tool),
                relation: None,
                label_contains: None,
                properties: None,
                max_results: None,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "JWT lib");
    }

    #[tokio::test]
    async fn test_graph_engine_export() {
        let store = SqliteStore::in_memory().unwrap();

        store.add_node(test_node("n1", "A")).await.unwrap();
        store.add_node(test_node("n2", "B")).await.unwrap();
        store
            .add_edge(test_edge("e1", "n1", "n2"))
            .await
            .unwrap();

        let exported = store.engine_export();
        assert_eq!(exported.metadata.node_count, 2);
        assert_eq!(exported.metadata.edge_count, 1);
    }
}
