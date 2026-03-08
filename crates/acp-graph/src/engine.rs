use std::collections::{HashMap, HashSet, VecDeque};

use acp_core::types::graph::*;
use acp_core::{AcpError, EntryId};

/// Entry in the adjacency list.
#[derive(Debug, Clone)]
struct AdjEntry {
    edge_id: String,
    target_id: String,
    relation: Relation,
    #[allow(dead_code)]
    weight: f64,
}

/// In-memory context graph engine with adjacency list indexing.
pub struct GraphEngine {
    nodes: HashMap<String, Node>,
    edges: HashMap<String, Edge>,
    adjacency: HashMap<String, Vec<AdjEntry>>,
    reverse: HashMap<String, Vec<AdjEntry>>,
    type_index: HashMap<NodeType, Vec<String>>,
    relation_index: HashMap<Relation, Vec<String>>,
}

impl Default for GraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphEngine {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            adjacency: HashMap::new(),
            reverse: HashMap::new(),
            type_index: HashMap::new(),
            relation_index: HashMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn get_edge(&self, id: &str) -> Option<&Edge> {
        self.edges.get(id)
    }

    pub fn add_node(&mut self, node: Node) -> Result<EntryId, AcpError> {
        let id = node.id.0.clone();

        self.type_index
            .entry(node.node_type)
            .or_default()
            .push(id.clone());

        self.adjacency.entry(id.clone()).or_default();
        self.reverse.entry(id.clone()).or_default();
        self.nodes.insert(id.clone(), node);

        Ok(EntryId(id))
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<EntryId, AcpError> {
        if !self.nodes.contains_key(&edge.source.0) {
            return Err(AcpError::EntryNotFound(edge.source.0.clone()));
        }
        if !self.nodes.contains_key(&edge.target.0) {
            return Err(AcpError::EntryNotFound(edge.target.0.clone()));
        }

        if matches!(edge.relation, Relation::DependsOn | Relation::BlockedBy)
            && self.would_create_cycle(&edge.source.0, &edge.target.0)
        {
            return Err(AcpError::GraphCycle);
        }

        let id = edge.id.0.clone();

        self.adjacency
            .entry(edge.source.0.clone())
            .or_default()
            .push(AdjEntry {
                edge_id: id.clone(),
                target_id: edge.target.0.clone(),
                relation: edge.relation,
                weight: edge.weight,
            });

        self.reverse
            .entry(edge.target.0.clone())
            .or_default()
            .push(AdjEntry {
                edge_id: id.clone(),
                target_id: edge.source.0.clone(),
                relation: edge.relation,
                weight: edge.weight,
            });

        self.relation_index
            .entry(edge.relation)
            .or_default()
            .push(id.clone());

        self.edges.insert(id.clone(), edge);

        Ok(EntryId(id))
    }

    pub fn remove_node(&mut self, id: &str) -> Result<(), AcpError> {
        let node = self
            .nodes
            .remove(id)
            .ok_or_else(|| AcpError::EntryNotFound(id.to_string()))?;

        if let Some(ids) = self.type_index.get_mut(&node.node_type) {
            ids.retain(|i| i != id);
        }

        if let Some(adj) = self.adjacency.remove(id) {
            for entry in &adj {
                if let Some(rev) = self.reverse.get_mut(&entry.target_id) {
                    rev.retain(|e| e.edge_id != entry.edge_id);
                }
                if let Some(rel_ids) = self.edges.get(&entry.edge_id).map(|e| e.relation) {
                    if let Some(idx) = self.relation_index.get_mut(&rel_ids) {
                        idx.retain(|i| i != &entry.edge_id);
                    }
                }
                self.edges.remove(&entry.edge_id);
            }
        }

        if let Some(rev) = self.reverse.remove(id) {
            for entry in &rev {
                if let Some(adj) = self.adjacency.get_mut(&entry.target_id) {
                    adj.retain(|e| e.edge_id != entry.edge_id);
                }
                if let Some(rel_ids) = self.edges.get(&entry.edge_id).map(|e| e.relation) {
                    if let Some(idx) = self.relation_index.get_mut(&rel_ids) {
                        idx.retain(|i| i != &entry.edge_id);
                    }
                }
                self.edges.remove(&entry.edge_id);
            }
        }

        Ok(())
    }

    pub fn remove_edge(&mut self, id: &str) -> Result<(), AcpError> {
        let edge = self
            .edges
            .remove(id)
            .ok_or_else(|| AcpError::EntryNotFound(id.to_string()))?;

        if let Some(adj) = self.adjacency.get_mut(&edge.source.0) {
            adj.retain(|e| e.edge_id != id);
        }
        if let Some(rev) = self.reverse.get_mut(&edge.target.0) {
            rev.retain(|e| e.edge_id != id);
        }
        if let Some(rel_ids) = self.relation_index.get_mut(&edge.relation) {
            rel_ids.retain(|i| i != id);
        }

        Ok(())
    }

    // ── Traversal ────────────────────────────────────────────

    pub fn traverse_bfs(
        &self,
        start: &str,
        relation: Option<Relation>,
        max_depth: u32,
    ) -> Vec<(&Node, u32)> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        visited.insert(start.to_string());
        queue.push_back((start.to_string(), 0u32));

        while let Some((node_id, depth)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            if let Some(node) = self.nodes.get(&node_id) {
                results.push((node, depth));
            }

            if let Some(adj) = self.adjacency.get(&node_id) {
                for entry in adj {
                    if let Some(ref rel) = relation {
                        if entry.relation != *rel {
                            continue;
                        }
                    }
                    if !visited.contains(&entry.target_id) {
                        visited.insert(entry.target_id.clone());
                        queue.push_back((entry.target_id.clone(), depth + 1));
                    }
                }
            }
        }

        results
    }

    pub fn nodes_by_type(&self, node_type: NodeType) -> Vec<&Node> {
        self.type_index
            .get(&node_type)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn edges_by_relation(&self, relation: Relation) -> Vec<&Edge> {
        self.relation_index
            .get(&relation)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn neighbors(&self, node_id: &str) -> Vec<(&Node, &Edge)> {
        self.adjacency
            .get(node_id)
            .map(|adj| {
                adj.iter()
                    .filter_map(|entry| {
                        let node = self.nodes.get(&entry.target_id)?;
                        let edge = self.edges.get(&entry.edge_id)?;
                        Some((node, edge))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── Subgraph ─────────────────────────────────────────────

    pub fn subgraph(&self, root: &str, depth: u32, max_nodes: u32) -> Result<SubGraph, AcpError> {
        if !self.nodes.contains_key(root) {
            return Err(AcpError::EntryNotFound(root.to_string()));
        }

        let mut visited_nodes = HashSet::new();
        let mut visited_edges = HashSet::new();
        let mut queue = VecDeque::new();

        visited_nodes.insert(root.to_string());
        queue.push_back((root.to_string(), 0u32));

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        while let Some((node_id, d)) = queue.pop_front() {
            if nodes.len() >= max_nodes as usize {
                break;
            }

            if let Some(node) = self.nodes.get(&node_id) {
                nodes.push(node.clone());
            }

            if d >= depth {
                continue;
            }

            if let Some(adj) = self.adjacency.get(&node_id) {
                for entry in adj {
                    if !visited_edges.contains(&entry.edge_id) {
                        visited_edges.insert(entry.edge_id.clone());
                        if let Some(edge) = self.edges.get(&entry.edge_id) {
                            edges.push(edge.clone());
                        }
                    }
                    if !visited_nodes.contains(&entry.target_id) {
                        visited_nodes.insert(entry.target_id.clone());
                        queue.push_back((entry.target_id.clone(), d + 1));
                    }
                }
            }

            if let Some(rev) = self.reverse.get(&node_id) {
                for entry in rev {
                    if !visited_edges.contains(&entry.edge_id) {
                        visited_edges.insert(entry.edge_id.clone());
                        if let Some(edge) = self.edges.get(&entry.edge_id) {
                            edges.push(edge.clone());
                        }
                    }
                    if !visited_nodes.contains(&entry.target_id) {
                        visited_nodes.insert(entry.target_id.clone());
                        queue.push_back((entry.target_id.clone(), d + 1));
                    }
                }
            }
        }

        Ok(SubGraph {
            nodes,
            edges,
            root: EntryId(root.to_string()),
            depth,
        })
    }

    // ── Cycle Detection & Shortest Path ──────────────────────

    pub fn would_create_cycle(&self, source: &str, target: &str) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![target.to_string()];

        while let Some(node_id) = stack.pop() {
            if node_id == source {
                return true;
            }
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id.clone());

            if let Some(adj) = self.adjacency.get(&node_id) {
                for entry in adj {
                    stack.push(entry.target_id.clone());
                }
            }
        }

        false
    }

    pub fn shortest_path(&self, from: &str, to: &str) -> Option<Vec<&Node>> {
        if from == to {
            return self.nodes.get(from).map(|n| vec![n]);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        visited.insert(from.to_string());
        queue.push_back(from.to_string());

        while let Some(node_id) = queue.pop_front() {
            if let Some(adj) = self.adjacency.get(&node_id) {
                for entry in adj {
                    if !visited.contains(&entry.target_id) {
                        visited.insert(entry.target_id.clone());
                        parent.insert(entry.target_id.clone(), node_id.clone());

                        if entry.target_id == to {
                            return Some(self.reconstruct_path(&parent, from, to));
                        }

                        queue.push_back(entry.target_id.clone());
                    }
                }
            }
        }

        None
    }

    fn reconstruct_path<'a>(
        &'a self,
        parent: &HashMap<String, String>,
        from: &str,
        to: &str,
    ) -> Vec<&'a Node> {
        let mut path = Vec::new();
        let mut current = to.to_string();

        while current != from {
            if let Some(node) = self.nodes.get(&current) {
                path.push(node);
            }
            current = parent[&current].clone();
        }

        if let Some(node) = self.nodes.get(from) {
            path.push(node);
        }

        path.reverse();
        path
    }

    // ── Serialization ────────────────────────────────────────

    pub fn export(&self) -> SerializedGraph {
        SerializedGraph {
            nodes: self.nodes.values().cloned().collect(),
            edges: self.edges.values().cloned().collect(),
            metadata: GraphMetadata {
                node_count: self.nodes.len(),
                edge_count: self.edges.len(),
                exported_at: chrono::Utc::now(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    pub fn import(data: &SerializedGraph) -> Result<Self, AcpError> {
        let mut engine = Self::new();
        for node in &data.nodes {
            engine.add_node(node.clone())?;
        }
        for edge in &data.edges {
            engine.add_edge(edge.clone())?;
        }
        Ok(engine)
    }

    pub fn to_json(&self) -> Result<String, AcpError> {
        serde_json::to_string_pretty(&self.export()).map_err(AcpError::Serialization)
    }

    pub fn from_json(json: &str) -> Result<Self, AcpError> {
        let data: SerializedGraph = serde_json::from_str(json).map_err(AcpError::Serialization)?;
        Self::import(&data)
    }

    // ── Query by pattern ─────────────────────────────────────

    pub fn query(&self, pattern: &GraphPattern) -> Vec<Node> {
        let mut results: Vec<&Node> = self.nodes.values().collect();

        if let Some(ref nt) = pattern.node_type {
            results.retain(|n| &n.node_type == nt);
        }

        if let Some(ref label) = pattern.label_contains {
            results.retain(|n| n.label.contains(label.as_str()));
        }

        if let Some(ref props) = pattern.properties {
            results.retain(|n| props.iter().all(|(k, v)| n.properties.get(k) == Some(v)));
        }

        if let Some(max) = pattern.max_results {
            results.truncate(max);
        }

        results.into_iter().cloned().collect()
    }
}

// ── Serialization types ──────────────────────────────────

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub metadata: GraphMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

// ── Merge types ──────────────────────────────────────────

#[derive(Debug)]
pub struct MergeResult {
    pub nodes_added: usize,
    pub nodes_merged: usize,
    pub edges_added: usize,
    pub conflicts: Vec<MergeConflict>,
}

#[derive(Debug)]
pub struct MergeConflict {
    pub node_id: String,
    pub field: String,
    pub local_value: String,
    pub remote_value: String,
}

#[derive(Debug, Clone, Copy)]
pub enum MergeStrategy {
    RemoteWins,
    LocalWins,
    MostRecent,
    ReportConflicts,
}

impl GraphEngine {
    pub fn merge(
        &mut self,
        other: &GraphEngine,
        strategy: MergeStrategy,
    ) -> Result<MergeResult, AcpError> {
        let mut result = MergeResult {
            nodes_added: 0,
            nodes_merged: 0,
            edges_added: 0,
            conflicts: Vec::new(),
        };

        for (id, remote_node) in &other.nodes {
            if let Some(local_node) = self.nodes.get(id) {
                match strategy {
                    MergeStrategy::RemoteWins => {
                        self.nodes.insert(id.clone(), remote_node.clone());
                        result.nodes_merged += 1;
                    }
                    MergeStrategy::LocalWins => {
                        result.nodes_merged += 1;
                    }
                    MergeStrategy::MostRecent => {
                        if remote_node.updated_at > local_node.updated_at {
                            self.nodes.insert(id.clone(), remote_node.clone());
                        }
                        result.nodes_merged += 1;
                    }
                    MergeStrategy::ReportConflicts => {
                        if local_node.label != remote_node.label {
                            result.conflicts.push(MergeConflict {
                                node_id: id.clone(),
                                field: "label".into(),
                                local_value: local_node.label.clone(),
                                remote_value: remote_node.label.clone(),
                            });
                        }
                    }
                }
            } else {
                self.add_node(remote_node.clone())?;
                result.nodes_added += 1;
            }
        }

        for (id, remote_edge) in &other.edges {
            if !self.edges.contains_key(id)
                && self.nodes.contains_key(&remote_edge.source.0)
                && self.nodes.contains_key(&remote_edge.target.0)
            {
                self.add_edge(remote_edge.clone())?;
                result.edges_added += 1;
            }
        }

        Ok(result)
    }
}
