use async_trait::async_trait;

use crate::types::*;
use crate::AcpError;

/// Trait for context graph operations (Conformance: Standard).
#[async_trait]
pub trait ContextGraphStore: Send + Sync {
    /// Add a node to the graph.
    async fn add_node(&self, node: Node) -> Result<NodeId, AcpError>;

    /// Add an edge between two nodes.
    async fn add_edge(&self, edge: Edge) -> Result<EdgeId, AcpError>;

    /// Query nodes matching a pattern.
    async fn query(&self, pattern: GraphPattern) -> Result<Vec<Node>, AcpError>;

    /// Extract a sub-graph from a root node.
    async fn subgraph(
        &self,
        root: &NodeId,
        depth: u32,
        max_nodes: u32,
    ) -> Result<SubGraph, AcpError>;

    /// Traverse the graph following a specific relation.
    async fn traverse(
        &self,
        start: &NodeId,
        relation: Relation,
        depth: u32,
    ) -> Result<Vec<Node>, AcpError>;

    /// Remove a node and its connected edges.
    async fn remove_node(&self, id: &NodeId) -> Result<(), AcpError>;

    /// Remove an edge.
    async fn remove_edge(&self, id: &EdgeId) -> Result<(), AcpError>;
}
