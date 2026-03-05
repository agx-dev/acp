use acp_core::*;
use serde_json::{json, Value};

use crate::server::AcpServer;

fn require_params(params: &Value) -> Result<&Value, AcpError> {
    if params.is_null() {
        Err(AcpError::InvalidParams("Missing params".into()))
    } else {
        Ok(params)
    }
}

impl AcpServer {
    /// Dispatch a JSON-RPC request to the appropriate handler.
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "acp.memory.store" => self.handle_memory_store(&request.params).await,
            "acp.memory.recall" => self.handle_memory_recall(&request.params).await,
            "acp.memory.forget" => self.handle_memory_forget(&request.params).await,
            "acp.memory.stats" => self.handle_memory_stats().await,

            "acp.context.addNode" => self.handle_context_add_node(request.params).await,
            "acp.context.addEdge" => self.handle_context_add_edge(request.params).await,
            "acp.context.query" => self.handle_context_query(request.params).await,
            "acp.context.subgraph" => self.handle_context_subgraph(&request.params).await,

            "acp.initialize" => self.handle_initialize().await,
            "acp.ping" => Ok(json!({"pong": true})),

            other => Err(AcpError::MethodNotFound(other.to_string())),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: Some(value),
                error: None,
                id: request.id,
            },
            Err(err) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(JsonRpcError {
                    code: err.code(),
                    message: err.to_string(),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    async fn handle_initialize(&self) -> Result<Value, AcpError> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false },
            },
            "serverInfo": {
                "name": "acp-server",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }))
    }

    async fn handle_memory_store(&self, params: &Value) -> Result<Value, AcpError> {
        let params = require_params(params)?;

        let content = params["content"]
            .as_str()
            .ok_or(AcpError::InvalidParams("Missing content".into()))?;

        let importance = params["importance"].as_f64().unwrap_or(0.7);
        let tags: Vec<String> = params["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let entry = SemanticEntry {
            id: EntryId::new("sem"),
            content: content.to_string(),
            embedding: None,
            source: types::semantic::SemanticSource::Manual,
            confidence: Confidence::new(0.9).unwrap(),
            importance,
            access_count: 0,
            last_accessed: None,
            tags,
            category: None,
            domain: None,
            protected: params["protected"].as_bool().unwrap_or(false),
            decay_rate: 0.01,
            provenance: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = self
            .store
            .store(Layer::Semantic, StoreEntry::Semantic(entry))
            .await?;

        Ok(json!({ "id": id.0 }))
    }

    async fn handle_memory_recall(&self, params: &Value) -> Result<Value, AcpError> {
        let text = params["query"].as_str().map(String::from);
        let top_k = params["top_k"].as_u64().map(|k| k as usize);

        let layers = params["layers"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| match v.as_str()? {
                        "episodic" => Some(Layer::Episodic),
                        "semantic" => Some(Layer::Semantic),
                        "graph" => Some(Layer::Graph),
                        "procedural" => Some(Layer::Procedural),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![Layer::Semantic]);

        let result = self
            .store
            .recall(RecallQuery {
                text,
                layers,
                top_k,
                ..Default::default()
            })
            .await?;

        Ok(json!({
            "entries": result.entries.iter().map(|e| json!({
                "id": e.id.0,
                "layer": format!("{:?}", e.layer),
                "content": e.content,
                "score": e.score,
                "tags": e.tags,
            })).collect::<Vec<_>>(),
            "total": result.entries.len(),
        }))
    }

    async fn handle_memory_forget(&self, params: &Value) -> Result<Value, AcpError> {
        let params = require_params(params)?;
        let id = params["id"]
            .as_str()
            .ok_or(AcpError::InvalidParams("Missing id".into()))?;

        self.store
            .forget(
                &EntryId(id.to_string()),
                types::retention::ForgetStrategy::Hard,
            )
            .await?;

        Ok(json!({ "deleted": true }))
    }

    async fn handle_memory_stats(&self) -> Result<Value, AcpError> {
        let stats = self
            .store
            .stats(&[
                Layer::Episodic,
                Layer::Semantic,
                Layer::Graph,
                Layer::Procedural,
            ])
            .await?;

        Ok(json!({
            "episodes": stats.episodes_count,
            "semantic": stats.semantic_count,
            "skills": stats.skills_count,
        }))
    }

    async fn handle_context_add_node(&self, params: Value) -> Result<Value, AcpError> {
        let node: types::graph::Node =
            serde_json::from_value(params).map_err(|e| AcpError::InvalidParams(e.to_string()))?;
        let id = self.graph.add_node(node).await?;
        Ok(json!({ "id": id.0 }))
    }

    async fn handle_context_add_edge(&self, params: Value) -> Result<Value, AcpError> {
        let edge: types::graph::Edge =
            serde_json::from_value(params).map_err(|e| AcpError::InvalidParams(e.to_string()))?;
        let id = self.graph.add_edge(edge).await?;
        Ok(json!({ "id": id.0 }))
    }

    async fn handle_context_query(&self, params: Value) -> Result<Value, AcpError> {
        let pattern: types::graph::GraphPattern =
            serde_json::from_value(params).map_err(|e| AcpError::InvalidParams(e.to_string()))?;
        let nodes = self.graph.query(pattern).await?;
        Ok(json!({ "nodes": nodes }))
    }

    async fn handle_context_subgraph(&self, params: &Value) -> Result<Value, AcpError> {
        let params = require_params(params)?;
        let root = params["root"]
            .as_str()
            .ok_or(AcpError::InvalidParams("Missing root".into()))?;
        let depth = params["depth"].as_u64().unwrap_or(2) as u32;
        let max_nodes = params["max_nodes"].as_u64().unwrap_or(50) as u32;

        let subgraph = self
            .graph
            .subgraph(&EntryId(root.to_string()), depth, max_nodes)
            .await?;

        Ok(json!(subgraph))
    }
}
