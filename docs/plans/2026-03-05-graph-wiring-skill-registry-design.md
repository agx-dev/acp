# Graph Wiring + SkillRegistry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the 3 remaining graph methods to the server handler, then implement SkillRegistry on SqliteStore with full CRUD + resolve.

**Architecture:** Direct wiring of existing GraphStore trait methods to JSON-RPC dispatch + MCP tools. SkillRegistry implemented on SqliteStore using existing skills table with FTS5 search.

**Tech Stack:** Rust, rusqlite, async-trait, serde_json, regex (new dep for skill trigger matching)

---

## Bug Fix: Missing skills_fts Table

The schema (`acp-store/src/schema.rs`) is missing the `skills_fts` FTS5 virtual table, but `recall_skills()` in `memory.rs:516` references it. This must be fixed first.

---

### Task 1: Fix missing skills_fts in schema

**Files:**
- Modify: `crates/acp-store/src/schema.rs:195` (after skills table definition)

**Step 1: Write a failing test**

Add to `crates/acp-store/src/lib.rs` tests module:

```rust
#[tokio::test]
async fn test_store_and_recall_skill() {
    use acp_core::types::skill::*;

    let store = SqliteStore::in_memory().unwrap();
    let skill = SkillObject {
        id: EntryId::new("skill"),
        name: "git-commit".to_string(),
        version: semver::Version::new(1, 0, 0),
        description: "Create well-formatted git commits".to_string(),
        instruction: "Run git add then git commit with a descriptive message".to_string(),
        trigger: SkillTrigger {
            patterns: vec![],
            context_conditions: vec![],
            explicit_invocation: true,
        },
        dependencies: SkillDependencies {
            tools_required: vec!["bash".to_string()],
            skills_required: vec![],
            min_context_window: None,
        },
        performance: SkillPerformance {
            invocation_count: 0,
            success_rate: 0.0,
            avg_tokens_per_use: 0.0,
            avg_latency_ms: 0.0,
            last_used: None,
        },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let id = store
        .store(Layer::Procedural, StoreEntry::Skill(skill))
        .await
        .unwrap();
    assert!(id.0.starts_with("skill-"));

    let result = store
        .recall(RecallQuery {
            text: Some("git commit".to_string()),
            layers: vec![Layer::Procedural],
            top_k: Some(5),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(result.entries.len(), 1);
    assert!(result.entries[0].content.contains("git-commit"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p acp-store test_store_and_recall_skill -- --nocapture`
Expected: FAIL — `no such table: skills_fts`

**Step 3: Add skills_fts to schema**

Add after the skills table (after line 195 in `schema.rs`):

```sql
-- FTS for skills
CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(
    name,
    description,
    instruction,
    content='skills',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS skills_fts_insert AFTER INSERT ON skills BEGIN
    INSERT INTO skills_fts(rowid, name, description, instruction)
    VALUES (new.rowid, new.name, new.description, new.instruction);
END;

CREATE TRIGGER IF NOT EXISTS skills_fts_delete AFTER DELETE ON skills BEGIN
    INSERT INTO skills_fts(skills_fts, rowid, name, description, instruction)
    VALUES ('delete', old.rowid, old.name, old.description, old.instruction);
END;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p acp-store test_store_and_recall_skill -- --nocapture`
Expected: PASS

**Step 5: Run full test suite**

Run: `cargo test`
Expected: 56 tests pass (55 existing + 1 new)

**Step 6: Commit**

```bash
git add crates/acp-store/src/schema.rs crates/acp-store/src/lib.rs
git commit -m "Fix missing skills_fts table in schema"
```

---

### Task 2: Wire graph traverse handler

**Files:**
- Modify: `crates/acp-server/src/mcp/handler.rs` (add handler + dispatch route)

**Step 1: Write the failing test**

Add to `crates/acp-server/src/main.rs` tests module:

```rust
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
    }).await;
    srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.context.addNode".into(),
        params: json!({
            "id": "t2", "node_type": "tool", "label": "JWT",
            "properties": {}, "episode_refs": [], "semantic_refs": [],
            "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
        }),
        id: Some(json!(2)),
    }).await;

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
    }).await;

    // Traverse
    let resp = srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.graph.traverse".into(),
        params: json!({
            "start": "t1",
            "relation": "used_for",
            "depth": 2
        }),
        id: Some(json!(4)),
    }).await;

    assert!(resp.error.is_none());
    let nodes = resp.result.unwrap()["nodes"].as_array().unwrap().clone();
    assert_eq!(nodes.len(), 2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p acp-server test_graph_traverse -- --nocapture`
Expected: FAIL — method not found

**Step 3: Add handler + dispatch**

In `handler.rs`, add dispatch route after line 35:
```rust
"acp.graph.traverse"   => self.handle_graph_traverse(&request.params).await,
```

Add handler method:
```rust
async fn handle_graph_traverse(&self, params: &Value) -> Result<Value, AcpError> {
    let params = require_params(params)?;
    let start = params["start"]
        .as_str()
        .ok_or(AcpError::InvalidParams("Missing start".into()))?;
    let relation: Relation = serde_json::from_value(
        params["relation"].clone()
    ).map_err(|e| AcpError::InvalidParams(e.to_string()))?;
    let depth = params["depth"].as_u64().unwrap_or(2) as u32;

    let nodes = self
        .graph
        .traverse(&EntryId(start.to_string()), relation, depth)
        .await?;

    Ok(json!({ "nodes": nodes }))
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p acp-server test_graph_traverse -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/acp-server/src/mcp/handler.rs crates/acp-server/src/main.rs
git commit -m "Wire graph traverse to handler"
```

---

### Task 3: Wire graph removeNode and removeEdge handlers

**Files:**
- Modify: `crates/acp-server/src/mcp/handler.rs`
- Modify: `crates/acp-server/src/main.rs` (tests)

**Step 1: Write failing tests**

Add to `crates/acp-server/src/main.rs` tests:

```rust
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
    }).await;

    let resp = srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.graph.removeNode".into(),
        params: json!({ "id": "n1" }),
        id: Some(json!(2)),
    }).await;

    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap()["removed"], true);
}

#[tokio::test]
async fn test_graph_remove_edge() {
    let srv = AcpServer::in_memory().unwrap();

    // Add 2 nodes + 1 edge
    srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.context.addNode".into(),
        params: json!({
            "id": "a", "node_type": "task", "label": "A",
            "properties": {}, "episode_refs": [], "semantic_refs": [],
            "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
        }),
        id: Some(json!(1)),
    }).await;
    srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.context.addNode".into(),
        params: json!({
            "id": "b", "node_type": "result", "label": "B",
            "properties": {}, "episode_refs": [], "semantic_refs": [],
            "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"
        }),
        id: Some(json!(2)),
    }).await;
    srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.context.addEdge".into(),
        params: json!({
            "id": "e1", "source": "a", "target": "b",
            "relation": "led_to", "weight": 1.0,
            "created_at": "2025-01-01T00:00:00Z"
        }),
        id: Some(json!(3)),
    }).await;

    let resp = srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.graph.removeEdge".into(),
        params: json!({ "id": "e1" }),
        id: Some(json!(4)),
    }).await;

    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap()["removed"], true);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p acp-server test_graph_remove -- --nocapture`
Expected: FAIL — method not found

**Step 3: Add handlers + dispatch**

In `handler.rs`, add dispatch routes after traverse route:
```rust
"acp.graph.removeNode" => self.handle_graph_remove_node(&request.params).await,
"acp.graph.removeEdge" => self.handle_graph_remove_edge(&request.params).await,
```

Add handler methods:
```rust
async fn handle_graph_remove_node(&self, params: &Value) -> Result<Value, AcpError> {
    let params = require_params(params)?;
    let id = params["id"]
        .as_str()
        .ok_or(AcpError::InvalidParams("Missing id".into()))?;
    self.graph.remove_node(&EntryId(id.to_string())).await?;
    Ok(json!({ "removed": true }))
}

async fn handle_graph_remove_edge(&self, params: &Value) -> Result<Value, AcpError> {
    let params = require_params(params)?;
    let id = params["id"]
        .as_str()
        .ok_or(AcpError::InvalidParams("Missing id".into()))?;
    self.graph.remove_edge(&EntryId(id.to_string())).await?;
    Ok(json!({ "removed": true }))
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p acp-server test_graph_remove -- --nocapture`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add crates/acp-server/src/mcp/handler.rs crates/acp-server/src/main.rs
git commit -m "Wire graph removeNode and removeEdge to handler"
```

---

### Task 4: Add MCP tool definitions for graph operations

**Files:**
- Modify: `crates/acp-server/src/mcp/tools.rs`
- Modify: `crates/acp-server/src/mcp/handler.rs` (mcp_tools_call dispatch)
- Modify: `crates/acp-server/src/main.rs` (update tool count test)

**Step 1: Write failing test**

Update `test_mcp_tools_definitions` in `main.rs`:
```rust
#[test]
fn test_mcp_tools_definitions() {
    let tools = crate::mcp::tools::mcp_tools();
    assert_eq!(tools.len(), 6); // was 3, now 6
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"acp_graph_traverse"));
    assert!(names.contains(&"acp_graph_remove_node"));
    assert!(names.contains(&"acp_graph_remove_edge"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p acp-server test_mcp_tools_definitions -- --nocapture`
Expected: FAIL — len is 3, not 6

**Step 3: Add tool definitions to tools.rs**

Append 3 new entries to the vec in `mcp_tools()`:

```rust
json!({
    "name": "acp_graph_traverse",
    "description": "Traverse the context graph following a specific relation type from a starting node.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "start": {
                "type": "string",
                "description": "Starting node ID"
            },
            "relation": {
                "type": "string",
                "description": "Relation type to follow",
                "enum": ["caused_by", "led_to", "triggered", "part_of", "contains",
                         "depends_on", "blocked_by", "supports", "contradicts",
                         "refined_by", "used_for", "created_by", "modified_by", "resolved_by"]
            },
            "depth": {
                "type": "integer",
                "description": "Maximum traversal depth",
                "default": 2
            }
        },
        "required": ["start", "relation"]
    }
}),
json!({
    "name": "acp_graph_remove_node",
    "description": "Remove a node and all its connected edges from the context graph.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "description": "Node ID to remove"
            }
        },
        "required": ["id"]
    }
}),
json!({
    "name": "acp_graph_remove_edge",
    "description": "Remove an edge from the context graph.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "description": "Edge ID to remove"
            }
        },
        "required": ["id"]
    }
}),
```

Also wire them in `mcp_tools_call` in handler.rs:
```rust
"acp_graph_traverse" => self.handle_graph_traverse(arguments).await,
"acp_graph_remove_node" => self.handle_graph_remove_node(arguments).await,
"acp_graph_remove_edge" => self.handle_graph_remove_edge(arguments).await,
```

Also update `test_mcp_tools_list` to expect 6 tools.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p acp-server -- --nocapture`
Expected: all server tests pass

**Step 5: Commit**

```bash
git add crates/acp-server/src/mcp/tools.rs crates/acp-server/src/mcp/handler.rs crates/acp-server/src/main.rs
git commit -m "Add MCP tool definitions for graph traverse/remove"
```

---

### Task 5: Implement SkillRegistry trait — register + get + list

**Files:**
- Create: `crates/acp-store/src/skills.rs`
- Modify: `crates/acp-store/src/lib.rs` (add `mod skills;`)

**Step 1: Write failing tests**

Add to `crates/acp-store/src/lib.rs` tests:

```rust
#[tokio::test]
async fn test_skill_register_and_get() {
    use acp_core::SkillRegistry;
    use acp_core::types::skill::*;

    let store = SqliteStore::in_memory().unwrap();
    let skill = SkillObject {
        id: EntryId::new("skill"),
        name: "code-review".to_string(),
        version: semver::Version::new(1, 0, 0),
        description: "Review code for quality issues".to_string(),
        instruction: "Analyze the diff and provide feedback".to_string(),
        trigger: SkillTrigger {
            patterns: vec![TriggerPattern {
                regex: r"review|check|inspect".to_string(),
                confidence_threshold: 0.7,
            }],
            context_conditions: vec![],
            explicit_invocation: false,
        },
        dependencies: SkillDependencies {
            tools_required: vec!["bash".to_string()],
            skills_required: vec![],
            min_context_window: None,
        },
        performance: SkillPerformance {
            invocation_count: 0,
            success_rate: 0.0,
            avg_tokens_per_use: 0.0,
            avg_latency_ms: 0.0,
            last_used: None,
        },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let id = store.register(skill.clone()).await.unwrap();
    assert!(id.0.starts_with("skill-"));

    let retrieved = store.get(&id).await.unwrap();
    assert_eq!(retrieved.name, "code-review");
    assert_eq!(retrieved.description, "Review code for quality issues");
    assert_eq!(retrieved.dependencies.tools_required, vec!["bash"]);
}

#[tokio::test]
async fn test_skill_list() {
    use acp_core::SkillRegistry;
    use acp_core::types::skill::*;

    let store = SqliteStore::in_memory().unwrap();

    let make_skill = |name: &str| SkillObject {
        id: EntryId::new("skill"),
        name: name.to_string(),
        version: semver::Version::new(1, 0, 0),
        description: format!("{} description", name),
        instruction: "do the thing".to_string(),
        trigger: SkillTrigger { patterns: vec![], context_conditions: vec![], explicit_invocation: true },
        dependencies: SkillDependencies { tools_required: vec![], skills_required: vec![], min_context_window: None },
        performance: SkillPerformance { invocation_count: 0, success_rate: 0.0, avg_tokens_per_use: 0.0, avg_latency_ms: 0.0, last_used: None },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    store.register(make_skill("skill-a")).await.unwrap();
    store.register(make_skill("skill-b")).await.unwrap();

    let all = store.list().await.unwrap();
    assert_eq!(all.len(), 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p acp-store test_skill_register -- --nocapture`
Expected: FAIL — SkillRegistry not implemented

**Step 3: Create skills.rs with register + get + list**

Create `crates/acp-store/src/skills.rs`:

```rust
use async_trait::async_trait;
use rusqlite::params;

use acp_core::types::skill::*;
use acp_core::{AcpError, EntryId, SkillId, SkillRegistry};

use crate::store::SqliteStore;

/// Helper to reconstruct a SkillObject from a rusqlite Row.
fn row_to_skill(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillObject> {
    let id: String = row.get("id")?;
    let name: String = row.get("name")?;
    let version_str: String = row.get("version")?;
    let description: String = row.get("description")?;
    let instruction: String = row.get("instruction")?;

    let trigger_patterns_json: String = row.get("trigger_patterns")?;
    let context_conditions_json: String = row.get("context_conditions")?;
    let explicit_invocation: bool = row.get("explicit_invocation")?;

    let tools_required_json: String = row.get("tools_required")?;
    let skills_required_json: String = row.get("skills_required")?;
    let min_context_window: Option<u32> = row.get("min_context_window")?;

    let invocation_count: i64 = row.get("invocation_count")?;
    let success_rate: f64 = row.get("success_rate")?;
    let avg_tokens_per_use: f64 = row.get("avg_tokens_per_use")?;
    let avg_latency_ms: f64 = row.get("avg_latency_ms")?;
    let last_used_str: Option<String> = row.get("last_used")?;
    let changelog_json: String = row.get("changelog")?;

    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    Ok(SkillObject {
        id: EntryId(id),
        name,
        version: semver::Version::parse(&version_str).unwrap_or(semver::Version::new(0, 0, 0)),
        description,
        instruction,
        trigger: SkillTrigger {
            patterns: serde_json::from_str(&trigger_patterns_json).unwrap_or_default(),
            context_conditions: serde_json::from_str(&context_conditions_json).unwrap_or_default(),
            explicit_invocation,
        },
        dependencies: SkillDependencies {
            tools_required: serde_json::from_str(&tools_required_json).unwrap_or_default(),
            skills_required: serde_json::from_str(&skills_required_json).unwrap_or_default(),
            min_context_window,
        },
        performance: SkillPerformance {
            invocation_count: invocation_count as u64,
            success_rate,
            avg_tokens_per_use,
            avg_latency_ms,
            last_used: last_used_str
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
        },
        changelog: serde_json::from_str(&changelog_json).unwrap_or_default(),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

#[async_trait]
impl SkillRegistry for SqliteStore {
    async fn register(&self, skill: SkillObject) -> Result<SkillId, AcpError> {
        self.store_skill(skill)
    }

    async fn get(&self, id: &SkillId) -> Result<SkillObject, AcpError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM skills WHERE id = ?1")
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        stmt.query_row(params![id.0], row_to_skill)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AcpError::SkillNotFound(id.0.clone())
                }
                other => AcpError::Internal(other.to_string()),
            })
    }

    async fn list(&self) -> Result<Vec<SkillObject>, AcpError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM skills ORDER BY name")
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_skill)
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }

    async fn update(&self, id: &SkillId, skill: SkillObject) -> Result<(), AcpError> {
        todo!() // Task 6
    }

    async fn resolve(&self, context: &SkillContext) -> Result<Vec<SkillMatch>, AcpError> {
        todo!() // Task 7
    }

    async fn export(&self, id: &SkillId) -> Result<PortableSkill, AcpError> {
        todo!() // Task 7
    }
}
```

Add `mod skills;` to `crates/acp-store/src/lib.rs` (after `mod store;`).

**Step 4: Run tests to verify they pass**

Run: `cargo test -p acp-store test_skill_register test_skill_list -- --nocapture`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add crates/acp-store/src/skills.rs crates/acp-store/src/lib.rs
git commit -m "Implement SkillRegistry register/get/list on SqliteStore"
```

---

### Task 6: Implement SkillRegistry update + export

**Files:**
- Modify: `crates/acp-store/src/skills.rs` (replace todo!() for update and export)
- Modify: `crates/acp-store/src/lib.rs` (add tests)

**Step 1: Write failing tests**

Add to `crates/acp-store/src/lib.rs` tests:

```rust
#[tokio::test]
async fn test_skill_update() {
    use acp_core::SkillRegistry;
    use acp_core::types::skill::*;

    let store = SqliteStore::in_memory().unwrap();
    let skill = SkillObject {
        id: EntryId::new("skill"),
        name: "old-name".to_string(),
        version: semver::Version::new(1, 0, 0),
        description: "old description".to_string(),
        instruction: "old instruction".to_string(),
        trigger: SkillTrigger { patterns: vec![], context_conditions: vec![], explicit_invocation: true },
        dependencies: SkillDependencies { tools_required: vec![], skills_required: vec![], min_context_window: None },
        performance: SkillPerformance { invocation_count: 0, success_rate: 0.0, avg_tokens_per_use: 0.0, avg_latency_ms: 0.0, last_used: None },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let id = store.register(skill).await.unwrap();

    let mut updated = store.get(&id).await.unwrap();
    updated.name = "new-name".to_string();
    updated.description = "new description".to_string();
    updated.version = semver::Version::new(2, 0, 0);

    store.update(&id, updated).await.unwrap();

    let retrieved = store.get(&id).await.unwrap();
    assert_eq!(retrieved.name, "new-name");
    assert_eq!(retrieved.description, "new description");
    assert_eq!(retrieved.version, semver::Version::new(2, 0, 0));
}

#[tokio::test]
async fn test_skill_export() {
    use acp_core::SkillRegistry;
    use acp_core::types::skill::*;

    let store = SqliteStore::in_memory().unwrap();
    let skill = SkillObject {
        id: EntryId::new("skill"),
        name: "exportable".to_string(),
        version: semver::Version::new(1, 0, 0),
        description: "A skill to export".to_string(),
        instruction: "do export stuff".to_string(),
        trigger: SkillTrigger { patterns: vec![], context_conditions: vec![], explicit_invocation: true },
        dependencies: SkillDependencies { tools_required: vec![], skills_required: vec![], min_context_window: None },
        performance: SkillPerformance { invocation_count: 5, success_rate: 0.9, avg_tokens_per_use: 100.0, avg_latency_ms: 50.0, last_used: None },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let id = store.register(skill).await.unwrap();
    let portable = store.export(&id).await.unwrap();
    assert_eq!(portable.skill.name, "exportable");
    assert!(portable.source_agent.is_none());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p acp-store test_skill_update test_skill_export -- --nocapture`
Expected: FAIL — panics on todo!()

**Step 3: Implement update + export**

Replace `todo!()` in `skills.rs`:

```rust
async fn update(&self, id: &SkillId, skill: SkillObject) -> Result<(), AcpError> {
    let conn = self.conn();
    let affected = conn
        .execute(
            "UPDATE skills SET
                name = ?2, version = ?3, description = ?4, instruction = ?5,
                trigger_patterns = ?6, context_conditions = ?7, explicit_invocation = ?8,
                tools_required = ?9, skills_required = ?10, min_context_window = ?11,
                invocation_count = ?12, success_rate = ?13, avg_tokens_per_use = ?14,
                avg_latency_ms = ?15, last_used = ?16, changelog = ?17,
                updated_at = datetime('now')
            WHERE id = ?1",
            params![
                id.0,
                skill.name,
                skill.version.to_string(),
                skill.description,
                skill.instruction,
                serde_json::to_string(&skill.trigger.patterns)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                serde_json::to_string(&skill.trigger.context_conditions)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                skill.trigger.explicit_invocation,
                serde_json::to_string(&skill.dependencies.tools_required)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                serde_json::to_string(&skill.dependencies.skills_required)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                skill.dependencies.min_context_window,
                skill.performance.invocation_count,
                skill.performance.success_rate,
                skill.performance.avg_tokens_per_use,
                skill.performance.avg_latency_ms,
                skill.performance.last_used.map(|d| d.to_rfc3339()),
                serde_json::to_string(&skill.changelog)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
            ],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

    if affected == 0 {
        return Err(AcpError::SkillNotFound(id.0.clone()));
    }
    Ok(())
}

async fn export(&self, id: &SkillId) -> Result<PortableSkill, AcpError> {
    let skill = self.get(id).await?;
    Ok(PortableSkill {
        skill,
        source_agent: None,
        exported_at: chrono::Utc::now(),
    })
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p acp-store test_skill_update test_skill_export -- --nocapture`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add crates/acp-store/src/skills.rs crates/acp-store/src/lib.rs
git commit -m "Implement SkillRegistry update/export"
```

---

### Task 7: Implement SkillRegistry resolve

**Files:**
- Modify: `crates/acp-store/src/skills.rs` (replace todo!() for resolve)
- Modify: `crates/acp-store/src/lib.rs` (add test)
- Modify: `crates/acp-store/Cargo.toml` (add `regex` dependency)

**Step 1: Write failing test**

Add to `crates/acp-store/src/lib.rs` tests:

```rust
#[tokio::test]
async fn test_skill_resolve() {
    use acp_core::SkillRegistry;
    use acp_core::types::skill::*;

    let store = SqliteStore::in_memory().unwrap();

    // Register a skill with trigger patterns
    let skill = SkillObject {
        id: EntryId::new("skill"),
        name: "code-review".to_string(),
        version: semver::Version::new(1, 0, 0),
        description: "Review code for quality".to_string(),
        instruction: "Analyze the diff".to_string(),
        trigger: SkillTrigger {
            patterns: vec![TriggerPattern {
                regex: r"(?i)review|check|inspect".to_string(),
                confidence_threshold: 0.7,
            }],
            context_conditions: vec![],
            explicit_invocation: false,
        },
        dependencies: SkillDependencies { tools_required: vec![], skills_required: vec![], min_context_window: None },
        performance: SkillPerformance { invocation_count: 10, success_rate: 0.85, avg_tokens_per_use: 0.0, avg_latency_ms: 0.0, last_used: None },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    store.register(skill).await.unwrap();

    // Register a non-matching skill
    let other = SkillObject {
        id: EntryId::new("skill"),
        name: "deploy".to_string(),
        version: semver::Version::new(1, 0, 0),
        description: "Deploy to production".to_string(),
        instruction: "Push and deploy".to_string(),
        trigger: SkillTrigger {
            patterns: vec![TriggerPattern {
                regex: r"(?i)deploy|ship|release".to_string(),
                confidence_threshold: 0.7,
            }],
            context_conditions: vec![],
            explicit_invocation: false,
        },
        dependencies: SkillDependencies { tools_required: vec![], skills_required: vec![], min_context_window: None },
        performance: SkillPerformance { invocation_count: 0, success_rate: 0.0, avg_tokens_per_use: 0.0, avg_latency_ms: 0.0, last_used: None },
        changelog: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    store.register(other).await.unwrap();

    let context = SkillContext {
        query: "Please review my code".to_string(),
        available_tools: vec![],
        session_tags: vec![],
    };

    let matches = store.resolve(&context).await.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].skill.name, "code-review");
    assert!(matches[0].confidence > 0.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p acp-store test_skill_resolve -- --nocapture`
Expected: FAIL — panics on todo!()

**Step 3: Add regex to Cargo.toml**

In `crates/acp-store/Cargo.toml`, add to `[dependencies]`:
```toml
regex = "1"
```

**Step 4: Implement resolve**

Replace `todo!()` in `skills.rs`:

```rust
async fn resolve(&self, context: &SkillContext) -> Result<Vec<SkillMatch>, AcpError> {
    let all_skills = self.list().await?;
    let mut matches = Vec::new();

    for skill in all_skills {
        if skill.trigger.explicit_invocation {
            continue;
        }

        let mut best_confidence = 0.0f64;
        let mut matched = false;

        for pattern in &skill.trigger.patterns {
            if let Ok(re) = regex::Regex::new(&pattern.regex) {
                if re.is_match(&context.query) {
                    matched = true;
                    best_confidence = best_confidence.max(pattern.confidence_threshold);
                }
            }
        }

        // If no patterns defined, try FTS-style name/description match
        if skill.trigger.patterns.is_empty() {
            let query_lower = context.query.to_lowercase();
            if skill.name.to_lowercase().contains(&query_lower)
                || skill.description.to_lowercase().contains(&query_lower)
            {
                matched = true;
                best_confidence = 0.5;
            }
        }

        if matched {
            // Boost confidence by past success rate
            let confidence = best_confidence * (0.5 + 0.5 * skill.performance.success_rate);

            matches.push(SkillMatch {
                skill,
                confidence,
                match_reason: format!("Pattern matched query: \"{}\"", context.query),
            });
        }
    }

    matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    Ok(matches)
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p acp-store test_skill_resolve -- --nocapture`
Expected: PASS

**Step 6: Run full suite**

Run: `cargo test`
Expected: all tests pass

**Step 7: Commit**

```bash
git add crates/acp-store/Cargo.toml crates/acp-store/src/skills.rs crates/acp-store/src/lib.rs
git commit -m "Implement SkillRegistry resolve with regex pattern matching"
```

---

### Task 8: Wire SkillRegistry to server handlers + MCP tools

**Files:**
- Modify: `crates/acp-server/src/mcp/handler.rs` (add skill handlers + dispatch)
- Modify: `crates/acp-server/src/mcp/tools.rs` (add skill MCP tools)
- Modify: `crates/acp-server/src/main.rs` (add tests, update tool counts)

**Step 1: Write failing tests**

Add to `crates/acp-server/src/main.rs` tests:

```rust
#[tokio::test]
async fn test_skill_register_and_list() {
    let srv = AcpServer::in_memory().unwrap();

    // Register
    let resp = srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.skill.register".into(),
        params: json!({
            "name": "test-skill",
            "version": "1.0.0",
            "description": "A test skill",
            "instruction": "Do the test thing",
            "trigger": { "patterns": [], "context_conditions": [], "explicit_invocation": true },
            "dependencies": { "tools_required": [], "skills_required": [] },
            "performance": { "invocation_count": 0, "success_rate": 0.0, "avg_tokens_per_use": 0.0, "avg_latency_ms": 0.0 },
            "changelog": []
        }),
        id: Some(json!(1)),
    }).await;
    assert!(resp.error.is_none());
    let id = resp.result.unwrap()["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("skill-"));

    // List
    let resp = srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.skill.list".into(),
        params: Value::Null,
        id: Some(json!(2)),
    }).await;
    assert!(resp.error.is_none());
    let skills = resp.result.unwrap()["skills"].as_array().unwrap().clone();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "test-skill");
}

#[tokio::test]
async fn test_skill_resolve_via_handler() {
    let srv = AcpServer::in_memory().unwrap();

    // Register a skill with trigger
    srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.skill.register".into(),
        params: json!({
            "name": "debugger",
            "version": "1.0.0",
            "description": "Debug runtime errors",
            "instruction": "Analyze stack trace",
            "trigger": {
                "patterns": [{ "regex": "(?i)debug|error|crash", "confidence_threshold": 0.8 }],
                "context_conditions": [],
                "explicit_invocation": false
            },
            "dependencies": { "tools_required": [], "skills_required": [] },
            "performance": { "invocation_count": 10, "success_rate": 0.9, "avg_tokens_per_use": 0.0, "avg_latency_ms": 0.0 },
            "changelog": []
        }),
        id: Some(json!(1)),
    }).await;

    let resp = srv.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "acp.skill.resolve".into(),
        params: json!({
            "query": "I have a debug error",
            "available_tools": [],
            "session_tags": []
        }),
        id: Some(json!(2)),
    }).await;

    assert!(resp.error.is_none());
    let matches = resp.result.unwrap()["matches"].as_array().unwrap().clone();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["skill"]["name"], "debugger");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p acp-server test_skill -- --nocapture`
Expected: FAIL — method not found

**Step 3: Add handlers + dispatch + MCP tools**

Add dispatch routes in handler.rs:
```rust
"acp.skill.register" => self.handle_skill_register(request.params).await,
"acp.skill.resolve"  => self.handle_skill_resolve(request.params).await,
"acp.skill.get"      => self.handle_skill_get(&request.params).await,
"acp.skill.update"   => self.handle_skill_update(&request.params).await,
"acp.skill.export"   => self.handle_skill_export(&request.params).await,
"acp.skill.list"     => self.handle_skill_list().await,
```

Add handler methods:
```rust
// ── ACP Skill Handlers ──────────────────────────────────

async fn handle_skill_register(&self, params: Value) -> Result<Value, AcpError> {
    use acp_core::SkillRegistry;
    let mut skill: SkillObject = serde_json::from_value(params)
        .map_err(|e| AcpError::InvalidParams(e.to_string()))?;
    skill.id = EntryId::new("skill");
    skill.created_at = chrono::Utc::now();
    skill.updated_at = chrono::Utc::now();
    let id = self.store.register(skill).await?;
    Ok(json!({ "id": id.0 }))
}

async fn handle_skill_resolve(&self, params: Value) -> Result<Value, AcpError> {
    use acp_core::SkillRegistry;
    let context: SkillContext = serde_json::from_value(params)
        .map_err(|e| AcpError::InvalidParams(e.to_string()))?;
    let matches = self.store.resolve(&context).await?;
    Ok(json!({ "matches": matches }))
}

async fn handle_skill_get(&self, params: &Value) -> Result<Value, AcpError> {
    use acp_core::SkillRegistry;
    let params = require_params(params)?;
    let id = params["id"]
        .as_str()
        .ok_or(AcpError::InvalidParams("Missing id".into()))?;
    let skill = self.store.get(&EntryId(id.to_string())).await?;
    Ok(serde_json::to_value(skill).map_err(|e| AcpError::Internal(e.to_string()))?)
}

async fn handle_skill_update(&self, params: &Value) -> Result<Value, AcpError> {
    use acp_core::SkillRegistry;
    let params = require_params(params)?;
    let id = params["id"]
        .as_str()
        .ok_or(AcpError::InvalidParams("Missing id".into()))?;
    let skill: SkillObject = serde_json::from_value(params["skill"].clone())
        .map_err(|e| AcpError::InvalidParams(e.to_string()))?;
    self.store.update(&EntryId(id.to_string()), skill).await?;
    Ok(json!({ "updated": true }))
}

async fn handle_skill_export(&self, params: &Value) -> Result<Value, AcpError> {
    use acp_core::SkillRegistry;
    let params = require_params(params)?;
    let id = params["id"]
        .as_str()
        .ok_or(AcpError::InvalidParams("Missing id".into()))?;
    let portable = self.store.export(&EntryId(id.to_string())).await?;
    Ok(serde_json::to_value(portable).map_err(|e| AcpError::Internal(e.to_string()))?)
}

async fn handle_skill_list(&self) -> Result<Value, AcpError> {
    use acp_core::SkillRegistry;
    let skills = self.store.list().await?;
    Ok(json!({ "skills": skills }))
}
```

Add `use acp_core::types::skill::SkillContext;` at top of handler.rs.

Add MCP tools in tools.rs (3 tools):
```rust
json!({
    "name": "acp_skill_register",
    "description": "Register a new skill in the agent's procedural memory.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "name": { "type": "string", "description": "Skill name" },
            "version": { "type": "string", "description": "Semver version" },
            "description": { "type": "string", "description": "What the skill does" },
            "instruction": { "type": "string", "description": "Execution instructions" }
        },
        "required": ["name", "version", "description", "instruction"]
    }
}),
json!({
    "name": "acp_skill_resolve",
    "description": "Find skills that match a given context or query.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Natural language query to match skills" },
            "available_tools": { "type": "array", "items": { "type": "string" }, "description": "Tools available in current session" },
            "session_tags": { "type": "array", "items": { "type": "string" }, "description": "Current session tags" }
        },
        "required": ["query"]
    }
}),
json!({
    "name": "acp_skill_list",
    "description": "List all registered skills.",
    "inputSchema": {
        "type": "object",
        "properties": {}
    }
}),
```

Wire in `mcp_tools_call`:
```rust
"acp_skill_register" => {
    let mut skill_params = arguments.clone();
    // Set defaults for MCP tool usage
    if skill_params.get("trigger").is_none() {
        skill_params["trigger"] = json!({ "patterns": [], "context_conditions": [], "explicit_invocation": true });
    }
    if skill_params.get("dependencies").is_none() {
        skill_params["dependencies"] = json!({ "tools_required": [], "skills_required": [] });
    }
    if skill_params.get("performance").is_none() {
        skill_params["performance"] = json!({ "invocation_count": 0, "success_rate": 0.0, "avg_tokens_per_use": 0.0, "avg_latency_ms": 0.0 });
    }
    if skill_params.get("changelog").is_none() {
        skill_params["changelog"] = json!([]);
    }
    self.handle_skill_register(skill_params).await
}
"acp_skill_resolve" => self.handle_skill_resolve(arguments.clone()).await,
"acp_skill_list" => self.handle_skill_list().await,
```

Update tool count assertions: `test_mcp_tools_definitions` → 9 tools, `test_mcp_tools_list` → 9 tools.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p acp-server -- --nocapture`
Expected: all server tests pass

**Step 5: Run full suite**

Run: `cargo test`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/acp-server/src/mcp/handler.rs crates/acp-server/src/mcp/tools.rs crates/acp-server/src/main.rs
git commit -m "Wire SkillRegistry to server handlers and MCP tools"
```

---

### Task 9: Update CLAUDE.md and rebuild server

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Update CLAUDE.md**

Update the "What's Left To Do" section:
- Mark graph methods as DONE
- Mark SkillRegistry as DONE
- Update test count
- Add skill MCP tools to the documented tools list

**Step 2: Rebuild server binary**

Run: `cargo build --release -p acp-server`
Expected: compiles without warnings

**Step 3: Run full suite one final time**

Run: `cargo test`
Expected: all tests pass (target: ~65+ tests)

**Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "Update CLAUDE.md with completed graph + skill features"
```
