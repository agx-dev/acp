<p align="center">
  <h1 align="center">ACP — Agent Context Protocol</h1>
  <p align="center">
    The open standard for agent memory, context, and cognitive state.<br/>
    <em>"The missing layer in the agentic AI protocol stack."</em>
  </p>
</p>

<p align="center">
  <a href="#the-problem">Problem</a> •
  <a href="#what-is-acp">What is ACP</a> •
  <a href="#protocol-stack">Protocol Stack</a> •
  <a href="#the-four-layer-memory-model">Memory Model</a> •
  <a href="#how-it-works">How it Works</a> •
  <a href="#roadmap">Roadmap</a>
</p>

---

## The Problem

Every AI agent framework implements memory differently. Knowledge dies with the framework. Skills can't be shared. Context is lost between sessions.

| Framework | Memory Approach | Limitation |
|-----------|----------------|------------|
| LangGraph | Reducer-based state | Framework-locked, no portability |
| CrewAI | SQLite short/long-term | Proprietary schema, no sharing |
| AutoGen | Message-list context | No persistent memory |
| Custom agents | Vector DB + prompts | No causality, no versioning |

Standards exist for **tools** (MCP), **agent communication** (A2A), and **web interaction** (WebMCP). But **no standard exists for agent memory and cognitive state**.

ACP fills that gap.

## What is ACP

**ACP (Agent Context Protocol)** is an open specification that defines how the memory, context, skills, and cognitive state of an AI agent are structured, versioned, indexed, and exchanged.

ACP is:
- **A protocol, not a product** — like HTTP, anyone can implement it
- **Model-agnostic** — works with Claude, GPT, Llama, or any LLM
- **Framework-agnostic** — not tied to LangChain, CrewAI, or any SDK
- **Wire-format compatible** — uses JSON-RPC 2.0 (same as MCP)
- **Designed for interoperability** — complements MCP, A2A, and WebMCP

## Protocol Stack

ACP occupies **Layer 4** in the emerging agentic AI protocol stack:

```
┌─────────────────────────────────────────────────────────────┐
│              THE AGENTIC AI PROTOCOL STACK                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Layer 4 ──  ACP    Agent Context Protocol    [MEMORY]      │
│              ↕ structure · version · share agent cognition  │
│                                                             │
│  Layer 3 ──  WebMCP  Agent-to-Web             [WEB]         │
│              ↕ W3C · Chrome                                 │
│                                                             │
│  Layer 2 ──  A2A    Agent-to-Agent            [COLLABORATION]│
│              ↕ Google · Linux Foundation                     │
│                                                             │
│  Layer 1 ──  MCP    Agent-to-Tool             [TOOLS]       │
│              ↕ Anthropic · Linux Foundation                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

Each layer solves a different problem. ACP is **complementary** — it doesn't replace MCP, A2A, or WebMCP.

## The Four-Layer Memory Model

ACP structures agent memory into four cognitive layers, inspired by neuroscience:

```
┌──────────────────────────────────────────────┐
│            EPISODIC MEMORY                   │
│  What happened? (events, conversations)      │
│  Append-only, immutable, timestamped         │
└─────────────────┬────────────────────────────┘
                  │ consolidation
┌─────────────────▼────────────────────────────┐
│            SEMANTIC MEMORY                    │
│  What do I know? (facts, patterns, rules)    │
│  Searchable by embedding, versioned          │
└─────────────────┬────────────────────────────┘
                  │ structuring
┌─────────────────▼────────────────────────────┐
│            CONTEXT GRAPH                      │
│  How do things relate? (causal graph)        │
│  Nodes, edges, traversal                     │
└─────────────────┬────────────────────────────┘
                  │ learning
┌─────────────────▼────────────────────────────┐
│            PROCEDURAL MEMORY                  │
│  How do I do things? (skills, routines)      │
│  Versioned, measurable, portable             │
└──────────────────────────────────────────────┘
```

**Key concept**: Knowledge flows **downward** through consolidation — raw episodes become structured knowledge, knowledge reveals relationships, relationships encode into reusable skills.

## How it Works

### 1. Store — Agent learns something

```json
{
  "method": "acp.memory.store",
  "params": {
    "layer": "semantic",
    "entry": {
      "content": "This project uses hexagonal architecture with ports in /domain/ports/",
      "tags": ["architecture", "convention"],
      "importance": 0.9
    }
  }
}
```

### 2. Recall — Agent retrieves knowledge

```json
{
  "method": "acp.memory.recall",
  "params": {
    "query": "project architecture",
    "layers": ["semantic", "procedural"],
    "top_k": 5
  }
}
```

### 3. Consolidate — Episodes become knowledge

Raw conversation episodes are automatically consolidated into structured semantic entries — similar to how human memory consolidation works during sleep.

### 4. Snapshot — Version the cognitive state

```json
{
  "method": "acp.version.snapshot",
  "params": {
    "reason": "End of debugging session — auth bug resolved"
  }
}
```

## Integration with Existing Protocols

ACP integrates seamlessly with the existing protocol stack:

```
┌──────────────────────────────────────────────┐
│              AI AGENT                         │
│                                              │
│  MCP tools:                                  │
│  ├── read_file, write_file, bash, ...        │
│  │                                           │
│  ACP tools (exposed as MCP):                 │
│  ├── acp_recall     (search memory)          │
│  ├── acp_store      (save knowledge)         │
│  ├── acp_skills     (find routines)          │
│  └── acp_snapshot   (version state)          │
└──────────────────────────────────────────────┘
```

The ACP server exposes itself as an **MCP server** — making it plug-and-play for any MCP-compatible agent (Claude Code, Codex, Cursor, etc.).

## Conformance Levels

ACP defines three conformance levels for progressive adoption:

| Level | What's Required | Use Case |
|-------|----------------|----------|
| **Core** | Episodic + Semantic memory, Store/Recall/Forget | Minimum viable memory |
| **Standard** | + Context Graph, + Version Store | Full cognitive architecture |
| **Full** | + Skill Registry, + Exchange, + A2A | Multi-agent collaboration |

## Reference Implementation

[**AGX (Agent Graph eXchange)**](https://github.com/agx-dev/agx) is the reference implementation of ACP. It provides:
- A portable `.agx` file format for packaging agents
- A runtime for executing agents
- A CLI (`agx`) for managing agents
- A registry for sharing agents

## Roadmap

- [x] Protocol specification (v0.1)
- [ ] Reference implementation (Rust)
- [ ] MCP bridge (ACP server as MCP server)
- [ ] Python SDK
- [ ] TypeScript SDK
- [ ] Public registry
- [ ] Formal RFC submission
- [ ] Academic paper

## Contributing

ACP is in early development. We welcome contributions, feedback, and discussion.

- Open an issue to discuss ideas
- See the [specification](docs/) for technical details (coming soon)

## License

Apache 2.0 — see [LICENSE](LICENSE)