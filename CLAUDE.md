# ACP — Agent Context Protocol

Open standard for persistent agent memory (Layer 4 of the agentic AI protocol stack).
GitHub: `agx-dev/acp` | License: Apache 2.0 | Rust workspace

## Architecture

```
acp-core          Types, traits, protocol, config (no runtime deps beyond serde/chrono)
acp-store         SQLite backend (rusqlite + FTS5), implements MemoryStore trait
acp-graph         In-memory graph engine (adjacency lists, BFS, merge), implements ContextGraphStore
acp-embeddings    Embedding abstraction (mock via SHA-256, OpenAI behind feature flag, LRU cache)
acp-server        Binary — assembles all crates, exposes via MCP (JSON-RPC over stdio)
```

## Commands

```bash
cargo test                              # 55 tests across 5 crates
cargo build --release -p acp-server     # Build the server binary
./target/release/acp-server --help      # CLI help
./target/release/acp-server stats --storage ~/.acp  # Show memory stats
```

## MCP Integration

The server speaks MCP protocol (stdio transport). Configured in `.mcp.json`:
```json
{
  "mcpServers": {
    "acp": {
      "command": "/Users/Apple/SelfProject/ACP/target/release/acp-server",
      "args": ["--storage", "/Users/Apple/.acp"]
    }
  }
}
```

MCP methods implemented: `initialize`, `notifications/initialized`, `ping`, `tools/list`, `tools/call`
Tools exposed: `acp_recall`, `acp_store`, `acp_context`

## Protocol Methods (AcpMethod enum)

23 methods defined in `acp-core/src/protocol/methods.rs`, grouped by conformance level:

### Core (implemented)
- `acp.memory.store` / `recall` / `forget` / `stats` — via acp-store (SQLite + FTS5)
- `acp.memory.prune` — trait + SQLite impl in MemoryStore::prune()

### Standard (partially implemented)
- `acp.context.addNode` / `addEdge` / `query` / `subgraph` — via acp-graph (in-memory)
- `acp.graph.traverse` / `removeNode` / `removeEdge` — engine methods exist, NOT wired to handler
- `acp.version.*` (snapshot/restore/diff/list) — trait defined, NO implementation yet

### Full (NOT implemented)
- `acp.skill.*` (register/resolve/get/update/export/list) — trait defined, NO handler
- `acp.exchange.*` (export/import) — trait defined, NO handler

## What's Left To Do

1. **Wire remaining graph methods** to the MCP handler:
   - `acp.graph.traverse` → `GraphEngine::traverse_bfs()`
   - `acp.graph.removeNode` → `GraphEngine::remove_node()`
   - `acp.graph.removeEdge` → `GraphEngine::remove_edge()`

2. **Implement SkillRegistry** for acp-store (SQLite skills table exists, CRUD not wired)

3. **Implement VersionManager** (snapshot/restore/diff of cognitive state)

4. **Implement Exchange** (export/import full agent bundles)

5. **Add MCP tools** for skills/versions/exchange when handlers are ready

6. **OpenAI embeddings** — `acp-embeddings` has the provider behind `openai` feature flag, needs env var config in server

7. **Graph persistence** — currently in-memory only, consider SQLite backing

8. **AGX reference implementation** — the `/Users/Apple/SelfProject/AGX` repo is the reference impl that uses ACP

## Known Patterns & Pitfalls

- **rusqlite lifetimes**: `MutexGuard<Connection>` must outlive `Statement` which must outlive `MappedRows`. Always bind `query_map()` result to a local `let rows = ...` before collecting.
- **Serde enum → SQL**: Use `serde_json::to_value().as_str()` (the `enum_to_sql` helper), NOT `serde_json::to_string()` which wraps values in JSON quotes.
- **FTS5 escaping**: User queries must be escaped via `fts5_escape()` to prevent operator injection (e.g. "end-to-end" would fail without quoting).
- **JSON-RPC params**: `JsonRpcRequest.params` is `serde_json::Value` with `#[serde(default)]`, NOT `Option<Value>`. Use `Value::Null` for empty params.
- **MCP notifications**: `notifications/initialized` has `id: None` — don't send a response for it.

## Preferences

- Short commit messages, no Co-Authored-By
- Multiple atomic commits (one per logical change)
- Multiple pushes (push after each crate/feature)
- French-speaking user
