//! End-to-end integration tests for the ACP MCP server.
//!
//! These tests exercise the full pipeline: MCP handshake, multiple tool calls,
//! file-backed persistence, and cross-layer operations.

use acp_core::*;
use serde_json::{json, Value};
use std::path::PathBuf;

mod helpers {
    use super::*;

    pub struct TestServer {
        inner: acp_server::AcpServer,
    }

    impl TestServer {
        pub fn in_memory() -> Self {
            Self {
                inner: acp_server::AcpServer::in_memory().unwrap(),
            }
        }

        pub fn with_storage(path: PathBuf) -> Self {
            let config = acp_server::ServerConfig {
                storage_path: path,
                embedding_provider: "mock".into(),
                openai_api_key: None,
                openai_model: "text-embedding-3-small".into(),
            };
            Self {
                inner: acp_server::AcpServer::with_config(config).unwrap(),
            }
        }

        pub async fn call(&self, method: &str, params: Value) -> Value {
            let resp = self
                .inner
                .handle_request(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    method: method.into(),
                    params,
                    id: Some(json!(1)),
                })
                .await;
            assert!(resp.error.is_none(), "RPC error: {:?}", resp.error);
            resp.result.unwrap()
        }

        pub async fn tool_call(&self, tool: &str, args: Value) -> Value {
            let result = self
                .call("tools/call", json!({ "name": tool, "arguments": args }))
                .await;
            let text = result["content"][0]["text"].as_str().unwrap();
            assert!(
                result.get("isError").is_none() || result["isError"] == false,
                "Tool error: {}",
                text,
            );
            serde_json::from_str(text).unwrap_or(json!({ "raw": text }))
        }
    }
}

use helpers::TestServer;

// ── Full MCP Session Flow ────────────────────────────────

#[tokio::test]
async fn test_full_mcp_session() {
    let srv = TestServer::in_memory();

    // Step 1: Initialize
    let init = srv
        .call(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "integration-test", "version": "1.0.0" }
            }),
        )
        .await;
    assert_eq!(init["protocolVersion"], "2024-11-05");
    assert_eq!(init["serverInfo"]["name"], "acp-server");

    // Step 2: Acknowledge initialization
    let srv_inner = &srv;
    let ack_resp = srv_inner
        .call("notifications/initialized", Value::Null)
        .await;
    assert!(ack_resp.is_object());

    // Step 3: List tools
    let tools = srv.call("tools/list", Value::Null).await;
    let tool_names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(tool_names.contains(&"acp_store"));
    assert!(tool_names.contains(&"acp_recall"));
    assert!(tool_names.contains(&"acp_context"));

    // Step 4: Store knowledge
    let stored = srv
        .tool_call(
            "acp_store",
            json!({
                "content": "Rust ownership prevents data races at compile time",
                "tags": ["rust", "safety"],
                "importance": 0.95
            }),
        )
        .await;
    assert!(stored["id"].as_str().unwrap().starts_with("sem-"));

    // Step 5: Recall knowledge
    let recalled = srv
        .tool_call(
            "acp_recall",
            json!({
                "query": "ownership data races",
                "layers": ["semantic"],
                "top_k": 5
            }),
        )
        .await;
    assert_eq!(recalled["total"], 1);
    assert!(recalled["entries"][0]["content"]
        .as_str()
        .unwrap()
        .contains("ownership"));
}

// ── File-Backed Persistence ──────────────────────────────

#[tokio::test]
async fn test_persistence_across_restarts() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().to_path_buf();

    // Session 1: Store data
    {
        let srv = TestServer::with_storage(path.clone());
        srv.tool_call(
            "acp_store",
            json!({
                "content": "Architecture uses hexagonal pattern",
                "importance": 0.9
            }),
        )
        .await;

        // Also store an episode
        srv.tool_call(
            "acp_store",
            json!({
                "content": "User asked about testing",
                "layer": "episodic",
                "role": "user",
                "session_id": "sess-1"
            }),
        )
        .await;
    }

    // Session 2: Data should survive
    {
        let srv = TestServer::with_storage(path.clone());

        let stats = srv.call("acp.memory.stats", Value::Null).await;
        assert_eq!(stats["semantic"], 1);
        assert_eq!(stats["episodes"], 1);

        let recalled = srv
            .tool_call(
                "acp_recall",
                json!({
                    "query": "hexagonal",
                    "layers": ["semantic"]
                }),
            )
            .await;
        assert_eq!(recalled["total"], 1);
    }
}

#[tokio::test]
async fn test_graph_persistence_across_restarts() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().to_path_buf();

    // Session 1: Build a graph
    {
        let srv = TestServer::with_storage(path.clone());

        srv.call(
            "acp.graph.add_node",
            json!({
                "id": "task-auth", "node_type": "task", "label": "Implement auth",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
        )
        .await;

        srv.call(
            "acp.graph.add_node",
            json!({
                "id": "tool-jwt", "node_type": "tool", "label": "JWT Library",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
        )
        .await;

        srv.call(
            "acp.graph.add_edge",
            json!({
                "id": "e-auth-jwt", "source": "task-auth", "target": "tool-jwt",
                "relation": "used_for", "weight": 1.0,
                "created_at": "2025-01-01T00:00:00Z"
            }),
        )
        .await;
    }

    // Session 2: Graph should be reloaded from SQLite
    {
        let srv = TestServer::with_storage(path.clone());

        let nodes = srv
            .call(
                "acp.graph.traverse",
                json!({
                    "start": "task-auth",
                    "relation": "used_for",
                    "depth": 1
                }),
            )
            .await;

        let node_list = nodes["nodes"].as_array().unwrap();
        assert_eq!(node_list.len(), 2);
    }
}

// ── Cross-Layer Workflow ─────────────────────────────────

#[tokio::test]
async fn test_cross_layer_workflow() {
    let srv = TestServer::in_memory();

    // Store semantic knowledge
    srv.tool_call(
        "acp_store",
        json!({
            "content": "Project uses event sourcing for audit trail",
            "tags": ["architecture"],
            "importance": 0.9
        }),
    )
    .await;

    // Store episodic memory
    srv.tool_call(
        "acp_store",
        json!({
            "content": "Discussed event sourcing with team lead",
            "layer": "episodic",
            "role": "agent",
            "session_id": "meeting-42"
        }),
    )
    .await;

    // Register a skill
    srv.tool_call(
        "acp_skill_register",
        json!({
            "id": "ignored",
            "name": "event-replay",
            "version": "1.0.0",
            "description": "Replay events from the event store",
            "instruction": "1. Connect to event store\n2. Select events by aggregate\n3. Replay in order",
            "trigger": { "patterns": [], "context_conditions": [], "explicit_invocation": true },
            "dependencies": { "tools_required": ["bash"], "skills_required": [], "min_context_window": null },
            "performance": { "invocation_count": 0, "success_rate": 0.0, "avg_tokens_per_use": 0.0, "avg_latency_ms": 0.0, "last_used": null },
            "changelog": [],
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z"
        }),
    )
    .await;

    // Build context graph
    srv.call(
        "acp.graph.add_node",
        json!({
            "id": "concept-es", "node_type": "knowledge", "label": "Event Sourcing",
            "properties": {}, "episode_refs": [], "semantic_refs": [],
            "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
        }),
    )
    .await;

    // Snapshot the full state
    let snapshot = srv
        .call(
            "acp.version.snapshot",
            json!({ "reason": "after architecture review", "layers": [], "tags": ["v1"] }),
        )
        .await;
    assert!(snapshot["id"].as_str().unwrap().starts_with("snap-"));

    // Export everything
    let bundle = srv.call("acp.exchange.export", Value::Null).await;
    assert_eq!(bundle["semantic_entries"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["episodes"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["skills"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["nodes"].as_array().unwrap().len(), 1);

    // Import into a fresh server
    let srv2 = TestServer::in_memory();
    let imported = srv2.call("acp.exchange.import", bundle).await;
    assert_eq!(imported["imported"]["semantic"], 1);
    assert_eq!(imported["imported"]["episodes"], 1);
    assert_eq!(imported["imported"]["skills"], 1);
    assert_eq!(imported["imported"]["nodes"], 1);

    // Verify data in the new server
    let stats = srv2.call("acp.memory.stats", Value::Null).await;
    assert_eq!(stats["semantic"], 1);
    assert_eq!(stats["episodes"], 1);
    assert_eq!(stats["skills"], 1);
}

// ── Prune + Snapshot/Restore Workflow ────────────────────

#[tokio::test]
async fn test_prune_and_restore_workflow() {
    let srv = TestServer::in_memory();

    // Store entries with varying importance
    for i in 0..5 {
        srv.tool_call(
            "acp_store",
            json!({
                "content": format!("Knowledge item {}", i),
                "importance": (i as f64) * 0.2 + 0.1, // 0.1, 0.3, 0.5, 0.7, 0.9
            }),
        )
        .await;
    }

    let stats = srv.call("acp.memory.stats", Value::Null).await;
    assert_eq!(stats["semantic"], 5);

    // Snapshot before prune
    let snap = srv
        .call(
            "acp.version.snapshot",
            json!({ "reason": "before prune", "layers": [], "tags": [] }),
        )
        .await;
    let snap_id = snap["id"].as_str().unwrap().to_string();

    // Prune low-importance entries (< 0.5)
    let pruned = srv
        .call(
            "acp.memory.prune",
            json!({
                "episodic": { "max_episodes": 10000, "max_age_days": 90, "eviction": "fifo" },
                "semantic": { "min_importance": 0.5 },
                "graph": { "prune_orphans": false }
            }),
        )
        .await;
    assert_eq!(pruned["semantic_pruned"], 2); // items 0 and 1

    let stats = srv.call("acp.memory.stats", Value::Null).await;
    assert_eq!(stats["semantic"], 3);

    // Restore snapshot to get all 5 back
    srv.call("acp.version.restore", json!({ "version": snap_id }))
        .await;

    let stats = srv.call("acp.memory.stats", Value::Null).await;
    assert_eq!(stats["semantic"], 5);
}

// ── Skill Lifecycle ──────────────────────────────────────

#[tokio::test]
async fn test_skill_full_lifecycle() {
    let srv = TestServer::in_memory();

    let skill_params = json!({
        "id": "ignored",
        "name": "deploy-prod",
        "version": "1.0.0",
        "description": "Deploy to production environment",
        "instruction": "1. Run tests\n2. Build release\n3. Push to registry\n4. Deploy",
        "trigger": {
            "patterns": [{ "regex": "deploy|ship|release", "confidence_threshold": 0.7 }],
            "context_conditions": [],
            "explicit_invocation": false
        },
        "dependencies": {
            "tools_required": ["bash", "docker"],
            "skills_required": [],
            "min_context_window": null
        },
        "performance": {
            "invocation_count": 0,
            "success_rate": 0.0,
            "avg_tokens_per_use": 0.0,
            "avg_latency_ms": 0.0,
            "last_used": null
        },
        "changelog": [],
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
    });

    // Register
    let registered = srv.call("acp.skill.register", skill_params.clone()).await;
    let skill_id = registered["id"].as_str().unwrap().to_string();

    // Get
    let skill = srv.call("acp.skill.get", json!({ "id": skill_id })).await;
    assert_eq!(skill["name"], "deploy-prod");
    assert_eq!(skill["dependencies"]["tools_required"][0], "bash");

    // List
    let list = srv.call("acp.skill.list", Value::Null).await;
    assert_eq!(list["total"], 1);

    // Update
    let mut updated = skill_params.clone();
    updated["id"] = json!(skill_id);
    updated["version"] = json!("2.0.0");
    updated["description"] = json!("Deploy to production with zero downtime");
    srv.call("acp.skill.update", updated).await;

    let skill = srv.call("acp.skill.get", json!({ "id": skill_id })).await;
    assert_eq!(skill["version"], "2.0.0");
    assert_eq!(
        skill["description"],
        "Deploy to production with zero downtime"
    );

    // Resolve
    let matches = srv
        .call(
            "acp.skill.resolve",
            json!({
                "query": "How do I deploy this service?",
                "available_tools": ["bash", "docker"],
                "session_tags": []
            }),
        )
        .await;
    assert!(matches["total"].as_u64().unwrap() >= 1);
    assert_eq!(matches["matches"][0]["skill"]["name"], "deploy-prod");

    // Export
    let portable = srv
        .call("acp.skill.export", json!({ "id": skill_id }))
        .await;
    assert_eq!(portable["skill"]["name"], "deploy-prod");
}
