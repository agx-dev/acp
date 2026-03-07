mod cli;
mod mcp;
mod server;
mod transport;

use acp_core::MemoryStore;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&args.log_level)),
        )
        .with_target(false)
        .init();

    tracing::info!("ACP Server v{}", env!("CARGO_PKG_VERSION"));

    let result = run(args).await;
    if let Err(e) = result {
        tracing::error!("Fatal: {}", e);
        std::process::exit(1);
    }
}

async fn run(args: cli::Cli) -> Result<(), acp_core::AcpError> {
    match args.command {
        None | Some(cli::Commands::Serve) => {
            let srv = server::AcpServer::new(args.storage)?;
            transport::stdio::serve_stdio(&srv).await?;
        }
        Some(cli::Commands::Stats) => {
            let srv = server::AcpServer::new(args.storage)?;
            let stats = srv
                .store
                .stats(&[
                    acp_core::Layer::Episodic,
                    acp_core::Layer::Semantic,
                    acp_core::Layer::Procedural,
                ])
                .await?;
            println!("Episodes:  {}", stats.episodes_count);
            println!("Semantic:  {}", stats.semantic_count);
            println!("Skills:    {}", stats.skills_count);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use acp_core::*;
    use serde_json::{json, Value};

    use crate::server::AcpServer;

    #[tokio::test]
    async fn test_ping() {
        let srv = AcpServer::in_memory().unwrap();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.ping".into(),
            params: Value::Null,
            id: Some(json!(1)),
        };
        let resp = srv.handle_request(req).await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["pong"], true);
    }

    #[tokio::test]
    async fn test_initialize() {
        let srv = AcpServer::in_memory().unwrap();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.initialize".into(),
            params: Value::Null,
            id: Some(json!(1)),
        };
        let resp = srv.handle_request(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], "acp-server");
    }

    #[tokio::test]
    async fn test_store_and_recall() {
        let srv = AcpServer::in_memory().unwrap();

        // Store
        let store_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({
                "content": "The project uses hexagonal architecture",
                "tags": ["architecture", "pattern"],
                "importance": 0.9,
            }),
            id: Some(json!(1)),
        };
        let store_resp = srv.handle_request(store_req).await;
        assert!(store_resp.error.is_none());
        let id = store_resp.result.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(id.starts_with("sem-"));

        // Recall
        let recall_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.recall".into(),
            params: json!({
                "query": "hexagonal",
                "layers": ["semantic"],
                "top_k": 5,
            }),
            id: Some(json!(2)),
        };
        let recall_resp = srv.handle_request(recall_req).await;
        assert!(recall_resp.error.is_none());
        let result = recall_resp.result.unwrap();
        assert_eq!(result["total"], 1);
        assert!(result["entries"][0]["content"]
            .as_str()
            .unwrap()
            .contains("hexagonal"));
    }

    #[tokio::test]
    async fn test_store_and_forget() {
        let srv = AcpServer::in_memory().unwrap();

        let store_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.store".into(),
                params: json!({ "content": "temporary data" }),
                id: Some(json!(1)),
            })
            .await;
        let id = store_resp.result.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let forget_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.forget".into(),
                params: json!({ "id": id }),
                id: Some(json!(2)),
            })
            .await;
        assert!(forget_resp.error.is_none());

        let stats_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(3)),
            })
            .await;
        assert_eq!(stats_resp.result.unwrap()["semantic"], 0);
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let srv = AcpServer::in_memory().unwrap();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.nonexistent".into(),
            params: Value::Null,
            id: Some(json!(1)),
        };
        let resp = srv.handle_request(req).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_stats_empty() {
        let srv = AcpServer::in_memory().unwrap();
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(1)),
            })
            .await;
        let result = resp.result.unwrap();
        assert_eq!(result["episodes"], 0);
        assert_eq!(result["semantic"], 0);
        assert_eq!(result["skills"], 0);
    }

    #[test]
    fn test_mcp_tools_definitions() {
        let tools = crate::mcp::tools::mcp_tools();
        assert_eq!(tools.len(), 19);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"acp_recall"));
        assert!(names.contains(&"acp_store"));
        assert!(names.contains(&"acp_context"));
        assert!(names.contains(&"acp_graph_traverse"));
        assert!(names.contains(&"acp_graph_remove_node"));
        assert!(names.contains(&"acp_graph_remove_edge"));
        assert!(names.contains(&"acp_skill_register"));
        assert!(names.contains(&"acp_skill_get"));
        assert!(names.contains(&"acp_skill_list"));
        assert!(names.contains(&"acp_skill_update"));
        assert!(names.contains(&"acp_skill_export"));
        assert!(names.contains(&"acp_skill_resolve"));
        assert!(names.contains(&"acp_version_snapshot"));
        assert!(names.contains(&"acp_version_restore"));
        assert!(names.contains(&"acp_version_diff"));
        assert!(names.contains(&"acp_version_list"));
        assert!(names.contains(&"acp_exchange_export"));
        assert!(names.contains(&"acp_exchange_import"));
        assert!(names.contains(&"acp_memory_prune"));
    }

    // ── MCP Protocol Tests ────────────────────────────────────

    #[tokio::test]
    async fn test_mcp_initialize() {
        let srv = AcpServer::in_memory().unwrap();
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "initialize".into(),
                params: json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "test-client", "version": "0.1.0" }
                }),
                id: Some(json!(1)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], "acp-server");
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let srv = AcpServer::in_memory().unwrap();
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "tools/list".into(),
                params: Value::Null,
                id: Some(json!(1)),
            })
            .await;
        assert!(resp.error.is_none());
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 19);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"acp_recall"));
        assert!(names.contains(&"acp_store"));
        assert!(names.contains(&"acp_context"));
        assert!(names.contains(&"acp_skill_register"));
        assert!(names.contains(&"acp_version_snapshot"));
        assert!(names.contains(&"acp_version_list"));
        assert!(names.contains(&"acp_exchange_export"));
        assert!(names.contains(&"acp_exchange_import"));
        assert!(names.contains(&"acp_memory_prune"));
    }

    #[tokio::test]
    async fn test_mcp_tools_call_store_and_recall() {
        let srv = AcpServer::in_memory().unwrap();

        // Store via tools/call
        let store_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "tools/call".into(),
                params: json!({
                    "name": "acp_store",
                    "arguments": {
                        "content": "Rust uses ownership for memory safety",
                        "tags": ["rust", "memory"],
                        "importance": 0.95
                    }
                }),
                id: Some(json!(1)),
            })
            .await;
        assert!(store_resp.error.is_none());
        let content = &store_resp.result.unwrap()["content"];
        assert_eq!(content[0]["type"], "text");
        assert!(!content[0]["text"].as_str().unwrap().contains("Error"));

        // Recall via tools/call
        let recall_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "tools/call".into(),
                params: json!({
                    "name": "acp_recall",
                    "arguments": {
                        "query": "ownership",
                        "layers": ["semantic"],
                        "top_k": 5
                    }
                }),
                id: Some(json!(2)),
            })
            .await;
        assert!(recall_resp.error.is_none());
        let text = recall_resp.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.contains("ownership"));
    }

    #[tokio::test]
    async fn test_mcp_tools_call_unknown_tool() {
        let srv = AcpServer::in_memory().unwrap();
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "tools/call".into(),
                params: json!({
                    "name": "nonexistent_tool",
                    "arguments": {}
                }),
                id: Some(json!(1)),
            })
            .await;
        // tools/call returns isError in content, not a JSON-RPC error
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], true);
    }

    #[tokio::test]
    async fn test_mcp_ping() {
        let srv = AcpServer::in_memory().unwrap();
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "ping".into(),
                params: Value::Null,
                id: Some(json!(1)),
            })
            .await;
        assert!(resp.error.is_none());
    }

    #[tokio::test]
    async fn test_mcp_notification_initialized() {
        let srv = AcpServer::in_memory().unwrap();
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "notifications/initialized".into(),
                params: Value::Null,
                id: None, // notifications have no id
            })
            .await;
        // Should not error — notifications are silently acknowledged
        assert!(resp.error.is_none());
    }

    // ── Skill Handler Tests ───────────────────────────────────

    fn sample_skill_params() -> Value {
        json!({
            "id": "ignored",
            "name": "test-skill",
            "version": "1.0.0",
            "description": "A test skill",
            "instruction": "Do the thing step by step",
            "trigger": {
                "patterns": [],
                "context_conditions": [],
                "explicit_invocation": true
            },
            "dependencies": {
                "tools_required": [],
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
        })
    }

    #[tokio::test]
    async fn test_skill_register_and_get() {
        let srv = AcpServer::in_memory().unwrap();

        // Register
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.register".into(),
                params: sample_skill_params(),
                id: Some(json!(1)),
            })
            .await;
        assert!(resp.error.is_none());
        let id = resp.result.unwrap()["id"].as_str().unwrap().to_string();
        assert!(id.starts_with("skill-"));

        // Get
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.get".into(),
                params: json!({ "id": id }),
                id: Some(json!(2)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["name"], "test-skill");
        assert_eq!(result["description"], "A test skill");
    }

    #[tokio::test]
    async fn test_skill_list() {
        let srv = AcpServer::in_memory().unwrap();

        // Register two skills
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.skill.register".into(),
            params: sample_skill_params(),
            id: Some(json!(1)),
        })
        .await;

        let mut params2 = sample_skill_params();
        params2["name"] = json!("skill-two");
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.skill.register".into(),
            params: params2,
            id: Some(json!(2)),
        })
        .await;

        // List
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.list".into(),
                params: Value::Null,
                id: Some(json!(3)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["total"], 2);
    }

    #[tokio::test]
    async fn test_skill_update_and_export() {
        let srv = AcpServer::in_memory().unwrap();

        // Register
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.register".into(),
                params: sample_skill_params(),
                id: Some(json!(1)),
            })
            .await;
        let id = resp.result.unwrap()["id"].as_str().unwrap().to_string();

        // Update
        let mut updated = sample_skill_params();
        updated["id"] = json!(id);
        updated["description"] = json!("Updated description");
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.update".into(),
                params: updated,
                id: Some(json!(2)),
            })
            .await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["updated"], true);

        // Export
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.export".into(),
                params: json!({ "id": id }),
                id: Some(json!(3)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["skill"]["description"], "Updated description");
        assert!(result["exported_at"].is_string());
    }

    #[tokio::test]
    async fn test_skill_resolve() {
        let srv = AcpServer::in_memory().unwrap();

        // Register a skill with a trigger pattern
        let mut params = sample_skill_params();
        params["name"] = json!("deploy-skill");
        params["description"] = json!("Deploy application to production");
        params["trigger"]["patterns"] = json!([
            { "regex": "deploy|deployment", "confidence_threshold": 0.8 }
        ]);
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.skill.register".into(),
            params,
            id: Some(json!(1)),
        })
        .await;

        // Resolve with matching query
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.skill.resolve".into(),
                params: json!({
                    "query": "How to deploy this app?",
                    "available_tools": [],
                    "session_tags": []
                }),
                id: Some(json!(2)),
            })
            .await;

        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(result["total"].as_u64().unwrap() >= 1);
        let first = &result["matches"][0];
        assert_eq!(first["skill"]["name"], "deploy-skill");
        assert!(first["confidence"].as_f64().unwrap() > 0.0);
    }

    // ── Version Handler Tests ─────────────────────────────────

    #[tokio::test]
    async fn test_version_snapshot_and_list() {
        let srv = AcpServer::in_memory().unwrap();

        // Store some data first
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({ "content": "test data" }),
            id: Some(json!(1)),
        })
        .await;

        // Create snapshot
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.snapshot".into(),
                params: json!({ "reason": "before refactor", "layers": [], "tags": ["test"] }),
                id: Some(json!(2)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(result["id"].as_str().unwrap().starts_with("snap-"));
        assert_eq!(result["stats"]["semantic_count"], 1);

        // List snapshots
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.list".into(),
                params: Value::Null,
                id: Some(json!(3)),
            })
            .await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["total"], 1);
    }

    #[tokio::test]
    async fn test_version_restore() {
        let srv = AcpServer::in_memory().unwrap();

        // Store data and snapshot
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({ "content": "original data" }),
            id: Some(json!(1)),
        })
        .await;

        let snap_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.snapshot".into(),
                params: json!({ "reason": "checkpoint", "layers": [], "tags": [] }),
                id: Some(json!(2)),
            })
            .await;
        let snap_id = snap_resp.result.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Add more data
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({ "content": "new data after snapshot" }),
            id: Some(json!(3)),
        })
        .await;

        // Verify 2 entries
        let stats = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(4)),
            })
            .await;
        assert_eq!(stats.result.unwrap()["semantic"], 2);

        // Restore
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.restore".into(),
                params: json!({ "version": snap_id }),
                id: Some(json!(5)),
            })
            .await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["restored"], true);

        // Verify only 1 entry remains
        let stats = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(6)),
            })
            .await;
        assert_eq!(stats.result.unwrap()["semantic"], 1);
    }

    #[tokio::test]
    async fn test_version_diff() {
        let srv = AcpServer::in_memory().unwrap();

        // Snapshot 1 — empty
        let snap1 = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.snapshot".into(),
                params: json!({ "reason": "empty", "layers": [], "tags": [] }),
                id: Some(json!(1)),
            })
            .await;
        let snap1_id = snap1.result.unwrap()["id"].as_str().unwrap().to_string();

        // Add data
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({ "content": "new knowledge" }),
            id: Some(json!(2)),
        })
        .await;

        // Snapshot 2 — with data
        let snap2 = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.snapshot".into(),
                params: json!({ "reason": "after learning", "layers": [], "tags": [] }),
                id: Some(json!(3)),
            })
            .await;
        let snap2_id = snap2.result.unwrap()["id"].as_str().unwrap().to_string();

        // Diff
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.version.diff".into(),
                params: json!({ "from": snap1_id, "to": snap2_id }),
                id: Some(json!(4)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["added"]["semantic_entries"], 1);
        assert_eq!(result["removed"]["semantic_entries"], 0);
    }

    // ── Exchange Handler Tests ────────────────────────────────

    #[tokio::test]
    async fn test_exchange_export_and_import() {
        let srv = AcpServer::in_memory().unwrap();

        // Store some data
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({ "content": "export test data", "importance": 0.8 }),
            id: Some(json!(1)),
        })
        .await;

        // Export
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.exchange.export".into(),
                params: Value::Null,
                id: Some(json!(2)),
            })
            .await;
        assert!(resp.error.is_none());
        let bundle = resp.result.unwrap();
        assert_eq!(bundle["semantic_entries"].as_array().unwrap().len(), 1);

        // Import into new server
        let srv2 = AcpServer::in_memory().unwrap();
        let resp = srv2
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.exchange.import".into(),
                params: bundle,
                id: Some(json!(3)),
            })
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["imported"]["semantic"], 1);

        // Verify data exists in new server
        let stats = srv2
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(4)),
            })
            .await;
        assert_eq!(stats.result.unwrap()["semantic"], 1);
    }

    // ── Episodic Store Test ───────────────────────────────────

    #[tokio::test]
    async fn test_store_episodic() {
        let srv = AcpServer::in_memory().unwrap();

        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.store".into(),
                params: json!({
                    "content": "User asked about hexagonal architecture",
                    "layer": "episodic",
                    "role": "user",
                    "session_id": "session-42",
                    "tags": ["architecture"],
                    "importance": 0.8
                }),
                id: Some(json!(1)),
            })
            .await;

        assert!(resp.error.is_none());
        let id = resp.result.unwrap()["id"].as_str().unwrap().to_string();
        assert!(id.starts_with("ep-"));

        // Verify it shows in stats
        let stats = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(2)),
            })
            .await;
        assert_eq!(stats.result.unwrap()["episodes"], 1);

        // Recall from episodic layer
        let recall = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.recall".into(),
                params: json!({
                    "query": "hexagonal",
                    "layers": ["episodic"],
                }),
                id: Some(json!(3)),
            })
            .await;
        assert!(recall.error.is_none());
        assert_eq!(recall.result.unwrap()["total"], 1);
    }

    // ── Memory Prune Test ────────────────────────────────────

    #[tokio::test]
    async fn test_memory_prune() {
        let srv = AcpServer::in_memory().unwrap();

        // Store a low-importance entry
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({
                "content": "low importance data",
                "importance": 0.05,
            }),
            id: Some(json!(1)),
        })
        .await;

        // Store a high-importance entry
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.memory.store".into(),
            params: json!({
                "content": "high importance data",
                "importance": 0.9,
            }),
            id: Some(json!(2)),
        })
        .await;

        // Prune with min_importance = 0.1
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.prune".into(),
                params: json!({
                    "episodic": { "max_episodes": 10000, "max_age_days": 90, "eviction": "fifo" },
                    "semantic": { "min_importance": 0.1 },
                    "graph": { "prune_orphans": false }
                }),
                id: Some(json!(3)),
            })
            .await;

        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["semantic_pruned"], 1);

        // Verify only high-importance entry remains
        let stats_resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.memory.stats".into(),
                params: Value::Null,
                id: Some(json!(4)),
            })
            .await;
        assert_eq!(stats_resp.result.unwrap()["semantic"], 1);
    }

    // ── Graph Traverse Test ─────────────────────────────────

    #[tokio::test]
    async fn test_graph_traverse() {
        let srv = AcpServer::in_memory().unwrap();

        // Add nodes
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addNode".into(),
            params: json!({
                "id": "t1", "node_type": "task", "label": "Auth",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(1)),
        })
        .await;
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addNode".into(),
            params: json!({
                "id": "t2", "node_type": "tool", "label": "JWT",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(2)),
        })
        .await;

        // Add edge
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addEdge".into(),
            params: json!({
                "id": "e1", "source": "t1", "target": "t2",
                "relation": "used_for", "weight": 1.0,
                "created_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(3)),
        })
        .await;

        // Traverse
        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.graph.traverse".into(),
                params: json!({
                    "start": "t1",
                    "relation": "used_for",
                    "depth": 2
                }),
                id: Some(json!(4)),
            })
            .await;

        assert!(resp.error.is_none());
        let nodes = resp.result.unwrap()["nodes"].as_array().unwrap().clone();
        assert_eq!(nodes.len(), 2);
    }

    // ── Graph Remove Tests ──────────────────────────────────

    #[tokio::test]
    async fn test_graph_remove_node() {
        let srv = AcpServer::in_memory().unwrap();

        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addNode".into(),
            params: json!({
                "id": "n1", "node_type": "task", "label": "Task",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(1)),
        })
        .await;

        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.graph.removeNode".into(),
                params: json!({ "id": "n1" }),
                id: Some(json!(2)),
            })
            .await;

        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["removed"], true);
    }

    #[tokio::test]
    async fn test_graph_remove_edge() {
        let srv = AcpServer::in_memory().unwrap();

        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addNode".into(),
            params: json!({
                "id": "a", "node_type": "task", "label": "A",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(1)),
        })
        .await;
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addNode".into(),
            params: json!({
                "id": "b", "node_type": "result", "label": "B",
                "properties": {}, "episode_refs": [], "semantic_refs": [],
                "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(2)),
        })
        .await;
        srv.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "acp.context.addEdge".into(),
            params: json!({
                "id": "e1", "source": "a", "target": "b",
                "relation": "led_to", "weight": 1.0,
                "created_at": "2025-01-01T00:00:00Z"
            }),
            id: Some(json!(3)),
        })
        .await;

        let resp = srv
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "acp.graph.removeEdge".into(),
                params: json!({ "id": "e1" }),
                id: Some(json!(4)),
            })
            .await;

        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["removed"], true);
    }
}
