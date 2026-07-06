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
Tools exposed: 28 tools — memory (`acp_recall`, `acp_store`, `acp_memory_prune`, `acp_memory_consolidate`), graph (`acp_context`, `acp_graph_traverse`, `acp_graph_merge`, `acp_graph_remove_node`, `acp_graph_remove_edge`), skills (`acp_skill_*`), versions (`acp_version_snapshot|restore|diff|list|branch|merge`), exchange (`acp_exchange_export|import|share|sync`), meta (`acp_capabilities`, `acp_health`)

## Protocol Methods (AcpMethod enum)

33 methods defined in `acp-core/src/protocol/methods.rs`, all wired to the handler and store. Grouped by conformance level:

### Wire namespace note
Graph ops use the **`acp.context.*`** namespace (canonical, per spec). The older
**`acp.graph.*`** names remain accepted as deprecated aliases (parse + dispatch)
but are NOT advertised in `acp.capabilities`.

### Core (implemented)
- `acp.memory.store` / `recall` / `forget` / `stats` / `prune` — via acp-store (SQLite + FTS5)
- `acp.exchange.export` / `import` — full agent bundles
- `acp.capabilities` / `acp.health`

### Standard (implemented)
- `acp.context.add_node` / `add_edge` / `query` / `subgraph` / `traverse` / `remove_node` / `remove_edge`
- `acp.version.snapshot` / `restore` / `diff` / `list`
- `acp.exchange.share` — layer- & tag-filtered selective share with access level
- `acp.memory.consolidate`

### Full (implemented)
- `acp.skill.register` / `resolve` / `get` / `update` / `export` / `list` / `invoke`
- `acp.context.merge` — merge external graph with conflict strategy + namespace
- `acp.version.branch` / `merge` — named branches over the snapshot store
- `acp.exchange.sync` — bidirectional sync (import peer bundle + return local bundle)

## Cross-cutting

- **Audit trail** (spec §11.4) — `handle_request` wraps `dispatch` and appends a
  best-effort row to the `audit_log` table for audited events (memory.store/recall/
  forget/consolidate, skill.invoke, version.snapshot/restore, exchange.share). See
  `acp-store/src/audit.rs` (`append_audit` / `query_audit`). Non-fatal on failure.
- **Snapshots version the graph** — `SnapshotData` captures full nodes/edges;
  `restore` re-inserts them and rebuilds the in-memory engine; `diff` counts graph deltas.
- **OpenAI embeddings** — `--embedding-provider openai` + `OPENAI_API_KEY`, compiled
  with `--features openai` (server crate forwards to `acp-embeddings/openai`). Without
  the feature, requesting `openai` returns a clear error (no silent fallback).

## What's Left To Do

Protocol is spec-complete (33/33 methods wired, 132 tests). Remaining items are enhancements, not gaps:

1. **True branch isolation** — `version.branch`/`merge` are named snapshot pointers (adopt-on-merge), not copy-on-write parallel timelines.
2. **Audit query RPC** — the audit trail is written + queryable at the store level (`query_audit`); not yet exposed as an `acp.audit.*` protocol method.
3. **Snapshot compression** — `compressed_bytes` equals `size_bytes` today (blobs are stored uncompressed).

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
