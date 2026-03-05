use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::EntryId;

/// A node in the context graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: EntryId,
    pub node_type: NodeType,
    pub label: String,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    pub embedding: Option<Vec<f32>>,
    #[serde(default)]
    pub episode_refs: Vec<EntryId>,
    #[serde(default)]
    pub semantic_refs: Vec<EntryId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Task,
    Decision,
    Tool,
    Result,
    Knowledge,
    Entity,
    Goal,
    Constraint,
    Event,
    Artifact,
}

/// An edge connecting two nodes in the context graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EntryId,
    pub source: EntryId,
    pub target: EntryId,
    pub relation: Relation,
    #[serde(default = "default_weight")]
    pub weight: f64,
    pub confidence: Option<f64>,
    pub evidence: Option<EntryId>,
    pub created_at: DateTime<Utc>,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    CausedBy,
    LedTo,
    Triggered,
    PartOf,
    Contains,
    DependsOn,
    BlockedBy,
    Supports,
    Contradicts,
    RefinedBy,
    UsedFor,
    CreatedBy,
    ModifiedBy,
    ResolvedBy,
}

/// A sub-graph extracted from the context graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub root: EntryId,
    pub depth: u32,
}

/// Pattern for querying the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    pub node_type: Option<NodeType>,
    pub relation: Option<Relation>,
    pub label_contains: Option<String>,
    pub properties: Option<HashMap<String, serde_json::Value>>,
    pub max_results: Option<usize>,
}
