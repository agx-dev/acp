//! # ACP Graph — Context Graph Engine
//!
//! In-memory directed graph with adjacency list indexing,
//! BFS/DFS traversal, subgraph extraction, cycle detection,
//! shortest path, and graph merge.

mod engine;

pub use engine::{GraphEngine, GraphMetadata, MergeConflict, MergeResult, MergeStrategy, SerializedGraph};

use std::sync::RwLock;

use async_trait::async_trait;

use acp_core::types::graph::*;
use acp_core::{AcpError, ContextGraphStore, EdgeId, NodeId};

/// Thread-safe wrapper around `GraphEngine` implementing ACP's `ContextGraphStore` trait.
pub struct GraphStore {
    engine: RwLock<GraphEngine>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            engine: RwLock::new(GraphEngine::new()),
        }
    }

    pub fn with_engine(engine: GraphEngine) -> Self {
        Self {
            engine: RwLock::new(engine),
        }
    }

    pub fn node_count(&self) -> usize {
        self.engine.read().unwrap().node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.engine.read().unwrap().edge_count()
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextGraphStore for GraphStore {
    async fn add_node(&self, node: Node) -> Result<NodeId, AcpError> {
        let mut engine = self
            .engine
            .write()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        engine.add_node(node)
    }

    async fn add_edge(&self, edge: Edge) -> Result<EdgeId, AcpError> {
        let mut engine = self
            .engine
            .write()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        engine.add_edge(edge)
    }

    async fn query(&self, pattern: GraphPattern) -> Result<Vec<Node>, AcpError> {
        let engine = self
            .engine
            .read()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        Ok(engine.query(&pattern))
    }

    async fn subgraph(
        &self,
        root: &NodeId,
        depth: u32,
        max_nodes: u32,
    ) -> Result<SubGraph, AcpError> {
        let engine = self
            .engine
            .read()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        engine.subgraph(&root.0, depth, max_nodes)
    }

    async fn traverse(
        &self,
        start: &NodeId,
        relation: Relation,
        depth: u32,
    ) -> Result<Vec<Node>, AcpError> {
        let engine = self
            .engine
            .read()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        let results = engine.traverse_bfs(&start.0, Some(relation), depth);
        Ok(results.into_iter().map(|(n, _)| n.clone()).collect())
    }

    async fn remove_node(&self, id: &NodeId) -> Result<(), AcpError> {
        let mut engine = self
            .engine
            .write()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        engine.remove_node(&id.0)
    }

    async fn remove_edge(&self, id: &EdgeId) -> Result<(), AcpError> {
        let mut engine = self
            .engine
            .write()
            .map_err(|_| AcpError::Internal("Lock poisoned".into()))?;
        engine.remove_edge(&id.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acp_core::EntryId;

    fn make_node(id: &str, node_type: NodeType, label: &str) -> Node {
        Node {
            id: EntryId(id.to_string()),
            node_type,
            label: label.to_string(),
            properties: Default::default(),
            embedding: None,
            episode_refs: vec![],
            semantic_refs: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_edge(id: &str, source: &str, target: &str, relation: Relation) -> Edge {
        Edge {
            id: EntryId(id.to_string()),
            source: EntryId(source.to_string()),
            target: EntryId(target.to_string()),
            relation,
            weight: 1.0,
            confidence: None,
            evidence: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn sample_graph() -> GraphEngine {
        let mut g = GraphEngine::new();

        g.add_node(make_node("task-1", NodeType::Task, "Implement auth"))
            .unwrap();
        g.add_node(make_node("tool-1", NodeType::Tool, "JWT Library"))
            .unwrap();
        g.add_node(make_node("result-1", NodeType::Result, "Auth working"))
            .unwrap();

        g.add_edge(make_edge("e-1", "task-1", "tool-1", Relation::UsedFor))
            .unwrap();
        g.add_edge(make_edge("e-2", "task-1", "result-1", Relation::LedTo))
            .unwrap();

        g
    }

    #[test]
    fn test_add_node_and_edge() {
        let g = sample_graph();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn test_subgraph() {
        let g = sample_graph();
        let sub = g.subgraph("task-1", 1, 10).unwrap();
        assert_eq!(sub.nodes.len(), 3);
        assert_eq!(sub.edges.len(), 2);
    }

    #[test]
    fn test_shortest_path() {
        let g = sample_graph();
        let path = g.shortest_path("task-1", "result-1").unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].id.0, "task-1");
        assert_eq!(path[1].id.0, "result-1");
    }

    #[test]
    fn test_cycle_detection() {
        let mut g = GraphEngine::new();

        g.add_node(make_node("a", NodeType::Task, "A")).unwrap();
        g.add_node(make_node("b", NodeType::Task, "B")).unwrap();

        g.add_edge(make_edge("e-ab", "a", "b", Relation::DependsOn))
            .unwrap();

        let result = g.add_edge(make_edge("e-ba", "b", "a", Relation::DependsOn));
        assert!(matches!(result, Err(AcpError::GraphCycle)));
    }

    #[test]
    fn test_remove_node() {
        let mut g = sample_graph();
        g.remove_node("tool-1").unwrap();
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1); // e-1 removed, e-2 remains
    }

    #[test]
    fn test_remove_edge() {
        let mut g = sample_graph();
        g.remove_edge("e-1").unwrap();
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.node_count(), 3);
    }

    #[test]
    fn test_nodes_by_type() {
        let g = sample_graph();
        let tasks = g.nodes_by_type(NodeType::Task);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "Implement auth");
    }

    #[test]
    fn test_neighbors() {
        let g = sample_graph();
        let neighbors = g.neighbors("task-1");
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_bfs_traversal() {
        let g = sample_graph();
        let results = g.traverse_bfs("task-1", None, 2);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_bfs_filtered_by_relation() {
        let g = sample_graph();
        let results = g.traverse_bfs("task-1", Some(Relation::UsedFor), 2);
        assert_eq!(results.len(), 2); // task-1 + tool-1
    }

    #[test]
    fn test_merge() {
        let mut g1 = sample_graph();
        let mut g2 = GraphEngine::new();

        g2.add_node(make_node("knowledge-1", NodeType::Knowledge, "JWT best practices"))
            .unwrap();

        let result = g1.merge(&g2, MergeStrategy::RemoteWins).unwrap();
        assert_eq!(result.nodes_added, 1);
        assert_eq!(g1.node_count(), 4);
    }

    #[test]
    fn test_json_roundtrip() {
        let g = sample_graph();
        let json = g.to_json().unwrap();
        let g2 = GraphEngine::from_json(&json).unwrap();
        assert_eq!(g2.node_count(), 3);
        assert_eq!(g2.edge_count(), 2);
    }

    #[test]
    fn test_query_by_type() {
        let g = sample_graph();
        let pattern = GraphPattern {
            node_type: Some(NodeType::Tool),
            relation: None,
            label_contains: None,
            properties: None,
            max_results: None,
        };
        let results = g.query(&pattern);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "JWT Library");
    }

    #[test]
    fn test_query_by_label() {
        let g = sample_graph();
        let pattern = GraphPattern {
            node_type: None,
            relation: None,
            label_contains: Some("auth".to_string()),
            properties: None,
            max_results: None,
        };
        let results = g.query(&pattern);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "Implement auth");
    }

    #[tokio::test]
    async fn test_graph_store_trait() {
        let store = GraphStore::new();

        let n1 = make_node("n1", NodeType::Task, "Task 1");
        let n2 = make_node("n2", NodeType::Result, "Result 1");

        store.add_node(n1).await.unwrap();
        store.add_node(n2).await.unwrap();

        let e1 = make_edge("e1", "n1", "n2", Relation::LedTo);
        store.add_edge(e1).await.unwrap();

        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);

        let nodes = store
            .traverse(&EntryId("n1".into()), Relation::LedTo, 1)
            .await
            .unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_edge_to_missing_node_fails() {
        let mut g = GraphEngine::new();
        g.add_node(make_node("a", NodeType::Task, "A")).unwrap();

        let result = g.add_edge(make_edge("e1", "a", "missing", Relation::LedTo));
        assert!(matches!(result, Err(AcpError::EntryNotFound(_))));
    }

    #[test]
    fn test_subgraph_missing_root_fails() {
        let g = GraphEngine::new();
        let result = g.subgraph("missing", 1, 10);
        assert!(matches!(result, Err(AcpError::EntryNotFound(_))));
    }
}
