# ACP — Agent Context Protocol

> The open standard for agent memory, context, and cognitive state.
> *"The missing layer in the agentic AI protocol stack."*

---

## 1 — Abstract

ACP (Agent Context Protocol) is an open specification defining how the memory, context, skills, and cognitive state of an AI agent are structured, versioned, indexed, and exchanged.

ACP addresses a critical gap in the current agentic AI ecosystem: while standards exist for agent-tool interaction (MCP), agent-agent communication (A2A), and agent-web interaction (WebMCP), **no standard exists for agent memory and context management**.

ACP is the theoretical and technical foundation behind [AGX (Agent Graph eXchange)](../../AGX/docs/README.md), its reference implementation.

---

## 2 — Motivation

### 2.1 — The Memory Problem

Every AI agent framework today implements memory differently:

| Framework | Memory Approach | Limitation |
|-----------|----------------|------------|
| LangGraph | Reducer-based state + external stores | Framework-locked, no portability |
| CrewAI | SQLite3 short/long-term memory | Scalability concerns, proprietary schema |
| AutoGen | Message-list context | No persistent memory, no structure |
| Custom agents | Vector database + raw prompts | Loss of causality, no versioning |

**Consequences of fragmentation:**
1. **No portability** — An agent's learned knowledge dies with its framework
2. **No reproducibility** — Cannot replay an agent's cognitive trajectory
3. **No sharing** — Cannot transfer skills or knowledge between agents
4. **No versioning** — Cannot rollback to a previous cognitive state
5. **No auditing** — Cannot trace how an agent reached a decision
6. **No interoperability** — Agents cannot share context across systems

### 2.2 — The Current Approaches and Their Limits

**Raw text / Prompt logs**
- Storage: conversation dumps, system prompt files
- Problem: inefficient search, expensive token usage, no structure

**Vector databases (semantic memory)**
- Storage: embeddings in Pinecone, Weaviate, ChromaDB
- Problem: loss of temporal ordering, loss of causality, loss of relationships

**State machine frameworks (orchestrated agents)**
- Storage: framework-specific state graphs
- Problem: not portable, not shareable, framework lock-in

**ACP combines the strengths of all three** while adding versioning, graph structure, and standardized operations.

### 2.3 — Position in the Protocol Stack

```
┌─────────────────────────────────────────────────────────────┐
│              THE AGENTIC AI PROTOCOL STACK                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Layer 4 ──  ACP    Agent Context Protocol    [MEMORY]      │
│              ↕ structure · version · share agent cognition  │
│                                                             │
│  Layer 3 ──  WebMCP  Agent-to-Web             [WEB]         │
│              ↕ W3C · Chrome 146                             │
│                                                             │
│  Layer 2 ──  A2A    Agent-to-Agent            [COLLABORATION│
│              ↕ Google · Linux Foundation · 150+ orgs        │
│                                                             │
│  Layer 1 ──  MCP    Agent-to-Tool             [TOOLS]       │
│              ↕ Anthropic · Linux Foundation · 97M+ DL/month │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

ACP is **complementary** to existing protocols. It does not replace MCP, A2A, or WebMCP. It fills the gap they leave: **memory and cognitive state**.

---

## 3 — Core Concepts

### 3.1 — The ACP Memory Model

ACP defines a **hybrid structured memory** inspired by cognitive neuroscience:

```
┌─────────────────────────────────────────────────────┐
│                 ACP MEMORY MODEL                     │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌───────────────┐    ┌───────────────┐             │
│  │   EPISODIC    │    │   SEMANTIC    │             │
│  │   MEMORY      │    │   MEMORY     │             │
│  │               │    │              │             │
│  │ Conversations │    │ Embeddings   │             │
│  │ Actions taken │    │ Knowledge    │             │
│  │ Results       │    │ Facts        │             │
│  │ Timestamps    │    │ Concepts     │             │
│  └───────┬───────┘    └──────┬───────┘             │
│          │                   │                      │
│          ▼                   ▼                      │
│  ┌───────────────────────────────────┐              │
│  │         GRAPH MEMORY             │              │
│  │                                  │              │
│  │  Nodes: entities, decisions,     │              │
│  │         tasks, tools, results    │              │
│  │  Edges: caused_by, used_for,    │              │
│  │         depends_on, led_to      │              │
│  └──────────────┬───────────────────┘              │
│                 │                                   │
│                 ▼                                   │
│  ┌───────────────────────────────────┐              │
│  │       PROCEDURAL MEMORY          │              │
│  │                                  │              │
│  │  Skills (versioned objects)      │              │
│  │  Learned routines               │              │
│  │  Tool usage patterns            │              │
│  └──────────────────────────────────┘              │
│                                                     │
│  ┌───────────────────────────────────┐              │
│  │       VERSION STORE              │              │
│  │  Git-like snapshots of all       │              │
│  │  memory layers                   │              │
│  └──────────────────────────────────┘              │
└─────────────────────────────────────────────────────┘
```

**Why four memory types?**

| Layer | Neuroscience Analogy | Function | Key Question |
|-------|---------------------|----------|--------------|
| **Episodic** | Hippocampus | Event recall | "What happened?" |
| **Semantic** | Neocortex | Factual knowledge | "What do I know?" |
| **Graph** | Prefrontal cortex | Causal reasoning | "How are things related?" |
| **Procedural** | Cerebellum | Learned skills | "How do I do this?" |

This is what differentiates ACP from flat vector databases. Memory is not a bag of vectors — it is a **cognitive architecture**.

### 3.2 — The Context Graph

The context graph is ACP's core innovation. It represents an agent's working context as a directed graph:

```
Context Graph
├── Nodes
│   ├── task:       "Resolve ticket #4521"
│   ├── decision:   "Escalate to human"
│   ├── tool:       "database-query"
│   ├── result:     "Customer found: John Doe"
│   ├── knowledge:  "SLA is 48h for premium"
│   └── entity:     "Customer: John Doe"
│
└── Edges (typed, weighted, timestamped)
    ├── caused_by:   decision → result
    ├── used_for:    tool → task
    ├── depends_on:  task → knowledge
    ├── led_to:      action → outcome
    ├── part_of:     subtask → task
    ├── contradicts: fact A → fact B
    ├── supports:    evidence → decision
    └── refined_by:  v1 → v2 of a conclusion
```

**What the context graph enables:**
- **Long-term reasoning**: Follow causal chains across sessions
- **Contextual navigation**: Find related information by traversing edges
- **Semantic recovery**: Reconstruct context from graph structure, not just embeddings
- **Provenance tracking**: Trace every decision back to its evidence

### 3.3 — Skills as Versioned Objects

Today, agent skills are informal — embedded in prompts, undocumented, unversioned. ACP proposes that skills become **first-class versioned objects**:

```
Skill Object
├── Identity (name, version, semver)
├── Instruction (the actual skill content)
├── Trigger conditions (when to activate)
├── Dependencies (required tools, other skills)
├── Performance metrics (success rate, token cost)
└── Changelog (version history)
```

Skills can be:
- **Versioned**: `code-debug@1.3.0`
- **Shared**: Published to a skill registry
- **Composed**: Skills can depend on other skills
- **Measured**: Performance tracked over time
- **Evolved**: Automatically updated based on performance data

### 3.4 — Memory Consolidation

Inspired by human memory consolidation during sleep, ACP defines a process for extracting durable knowledge from raw experience:

```
    EPISODIC                          SEMANTIC
    (raw events)                      (extracted knowledge)

    ┌──────────────┐                  ┌──────────────────┐
    │ ep-001: User │                  │                  │
    │ asked about  │──┐               │  "Return policy  │
    │ return policy│  │               │   is 30 days for │
    │              │  │   extract     │   electronics,   │
    │ ep-002: Agent│  ├──────────►   │   90 days for    │
    │ looked up DB │  │   patterns    │   clothing"      │
    │              │  │               │                  │
    │ ep-003: Agent│  │               │  confidence: 0.94│
    │ answered:    │──┘               │  source: [ep-001,│
    │ "30 days..." │                  │   ep-002, ep-003]│
    └──────────────┘                  └──────────────────┘
           │                                  │
           ▼                                  ▼
    ┌──────────────┐                  ┌──────────────────┐
    │ GRAPH NODE   │                  │ PROCEDURAL SKILL │
    │              │                  │                  │
    │ [return-     │                  │ skill: lookup-   │
    │  policy]     │                  │  return-policy   │
    │   ├── caused │                  │ steps:           │
    │   │   by:    │                  │  1. check product│
    │   │   user   │                  │     category     │
    │   │   query  │                  │  2. query DB     │
    │   └── led to:│                  │  3. format answer│
    │       answer │                  │ success_rate: 95%│
    └──────────────┘                  └──────────────────┘
```

**Consolidation is not just compression.** It is the transformation of experience into knowledge, routines, and causal understanding.

### 3.5 — Memory Retention (Structured Forgetting)

Forgetting is a feature, not a bug. Agents that never forget accumulate noise, degrade performance, and increase costs. ACP formalizes forgetting as a controlled process:

| Strategy | Mechanism | Use Case |
|----------|-----------|----------|
| **Decay** | Importance decreases exponentially over time | Episodic memory |
| **FIFO** | Oldest entries removed first | Log-like data |
| **Importance** | Low-importance entries pruned | Semantic memory |
| **Reachability** | Orphaned graph nodes removed | Context graph |

**Protected memories**: Entries tagged as `critical` or `user-preference` are never forgotten, regardless of policy.

---

## 4 — Formal Specification (RFC-style)

The following schemas use RFC 2119 keywords: MUST, SHOULD, MAY, MUST NOT.

### 4.1 — Agent Identity Schema

An ACP-compliant agent MUST declare its identity:

```yaml
acp_version: "1.0.0"                    # MUST specify ACP version
agent:
  id: "urn:acp:agent:<uuid>"            # MUST be globally unique URN
  name: "<human-readable-name>"         # MUST be human-readable
  version: "<semver>"                   # MUST follow Semantic Versioning
  created_at: "<ISO-8601>"              # MUST be ISO 8601 timestamp
  author: "<org:name | user:name>"      # SHOULD identify creator
  description: "<text>"                 # SHOULD describe purpose

  capabilities:                         # MUST declare capabilities
    - "text-generation"
    - "tool-use"
    - "multi-turn-conversation"
    - "code-execution"
    - "vision"

  model_requirements:                   # MUST specify model constraints
    min_context_window: 128000          # MUST specify minimum
    features_required:                  # MUST list required features
      - "function-calling"
      - "structured-output"
    model_bindings:                     # MAY specify preferred models
      - provider: "anthropic"
        model: "claude-sonnet-4-6"
        priority: 1
      - provider: "openai"
        model: "gpt-4o"
        priority: 2
      - provider: "local"
        model: "llama-3.1-70b"
        priority: 3

  protocols:                            # MUST declare supported protocols
    - "acp/1.0"
    - "mcp/1.0"                         # MAY support MCP
    - "a2a/1.0"                         # MAY support A2A
```

### 4.2 — Episodic Memory Schema

```yaml
episodic_memory:
  max_episodes: 10000                   # MAY set retention limit
  retention_policy: "decay"             # MUST define retention strategy

  episodes:                             # MUST be ordered by timestamp
    - id: "ep-<uuid>"                   # MUST be unique identifier
      timestamp: "<ISO-8601>"           # MUST be ISO 8601
      sequence_number: 1                # MUST be monotonically increasing

      type: "conversation |             # MUST be one of these types
              action |
              observation |
              reflection |
              error"

      content:
        role: "user | agent |           # MUST specify role
               system | tool"
        text: "<content>"               # MUST contain text content
        tokens_used: 1523               # SHOULD track token usage

      context_ref: "node-<uuid>"        # SHOULD reference context graph

      outcome:                          # MAY specify outcome
        status: "success |
                 failure |
                 partial"
        confidence: 0.92                # MAY specify confidence [0.0-1.0]

      metadata:                         # MAY include metadata
        session_id: "sess-<uuid>"
        trigger: "user_input |
                  scheduled |
                  event |
                  consolidation"
        parent_episode: "ep-<uuid>"     # MAY reference parent
        importance: 0.75                # MAY specify importance [0.0-1.0]
```

### 4.3 — Semantic Memory Schema

```yaml
semantic_memory:
  embedding_model: "<model-id>"         # MUST specify embedding model
  embedding_dimensions: 3072            # MUST specify dimensions
  distance_metric: "cosine |            # MUST specify distance metric
                    euclidean |
                    dot_product"

  entries:
    - id: "sem-<uuid>"                  # MUST be unique
      content: "<text>"                 # MUST contain text
      embedding: [0.012, -0.034, ...]   # MUST be float array

      source: "ep-<uuid> |             # MUST specify origin
               external |
               manual |
               consolidated"

      confidence: 0.95                  # MUST specify confidence [0.0-1.0]
      created_at: "<ISO-8601>"          # MUST be timestamped
      last_accessed: "<ISO-8601>"       # SHOULD track access
      access_count: 47                  # SHOULD track frequency

      tags: ["<tag>", ...]              # MAY include tags

      decay_rate: 0.001                 # MAY specify forgetting rate

      provenance:                       # SHOULD track provenance
        source_episodes: ["ep-001"]     # Episodes that generated this
        consolidation_id: "cons-<uuid>" # Consolidation batch
        verified: true                  # Human-verified?
```

### 4.4 — Context Graph Schema

```yaml
context_graph:
  format: "acp-graph-v1"               # MUST specify format

  nodes:
    - id: "node-<uuid>"                # MUST be unique
      type: "task |                     # MUST be typed
             decision |
             tool |
             result |
             knowledge |
             entity |
             goal |
             constraint"

      label: "<human-readable>"         # MUST have label

      properties:                       # MAY have properties
        status: "pending |
                 active |
                 completed |
                 failed |
                 abandoned"
        priority: "critical |
                   high |
                   medium |
                   low"
        created_at: "<ISO-8601>"
        updated_at: "<ISO-8601>"

      embedding:                        # MAY include embedding
        vector: [0.012, ...]
        model: "<embedding-model>"

  edges:
    - id: "edge-<uuid>"                # MUST be unique
      source: "node-<uuid>"            # MUST reference valid node
      target: "node-<uuid>"            # MUST reference valid node

      relation: "caused_by |           # MUST be typed
                 used_for |
                 depends_on |
                 led_to |
                 part_of |
                 contradicts |
                 supports |
                 refined_by |
                 blocked_by |
                 created_by"

      weight: 0.85                     # MAY specify strength [0.0-1.0]

      metadata:
        created_at: "<ISO-8601>"       # MUST be timestamped
        evidence: "ep-<uuid>"          # SHOULD link to evidence
        confidence: 0.9                # MAY specify confidence
```

### 4.5 — Skill Object Schema

```yaml
skills:
  - id: "skill-<uuid>"                 # MUST be unique
    name: "<skill-name>"               # MUST be human-readable
    version: "<semver>"                 # MUST follow semver
    description: "<text>"              # MUST describe the skill

    # Skill content
    instruction: |                      # MUST contain instructions
      <The actual skill content —
       what the agent should do
       when this skill is activated>

    # Activation
    trigger_conditions:                 # MUST define when to activate
      - pattern: "<regex>"             # Text pattern matching
        confidence_threshold: 0.7      # Minimum confidence to trigger
      - context_type: "<node-type>"    # Graph node type matching
      - explicit: true                 # Manual invocation only

    # Dependencies
    dependencies:                       # SHOULD declare dependencies
      tools_required: ["<tool>"]       # Required MCP tools
      skills_required: ["<skill>"]     # Required other skills
      min_context_window: 32000        # Minimum context window

    # Performance tracking
    performance:                        # SHOULD track performance
      invocation_count: 234
      success_rate: 0.87
      avg_tokens_per_use: 4500
      avg_latency_ms: 2300
      last_used: "<ISO-8601>"

    # Version history
    changelog:                          # SHOULD maintain changelog
      - version: "<semver>"
        date: "<ISO-8601>"
        changes: "<description>"
```

### 4.6 — Version Store Schema

```yaml
version_store:
  current_version: "v<n>"              # MUST track current version

  snapshots:
    - version: "v<n>"                  # MUST be sequential
      timestamp: "<ISO-8601>"          # MUST be timestamped
      parent: "v<n-1>"                 # MUST reference parent (except v1)
      hash: "sha256:<hash>"            # MUST be content-addressable

      delta_type: "full |              # MUST specify snapshot type
                   incremental"

      layers_included:                 # MUST specify which layers
        episodic: true
        semantic: true
        graph: true
        procedural: true

      size_bytes: 245760               # SHOULD track size

      trigger: "auto |                 # SHOULD specify trigger
                manual |
                checkpoint |
                pre_migration |
                error_recovery"

      metadata:
        session_id: "sess-<uuid>"
        reason: "<text>"               # SHOULD explain why
        agent_version: "<semver>"      # Agent version at snapshot time
```

### 4.7 — Retention Policy Schema

```yaml
retention:
  episodic:
    strategy: "decay |                 # MUST define strategy
               fifo |
               importance"
    max_episodes: 10000                # MUST set maximum
    decay_function: "exponential |     # Required if strategy=decay
                     linear |
                     step"
    decay_rate: 0.001                  # Rate per hour
    min_importance: 0.1                # Below = eligible for removal
    protected_tags: ["critical",       # MUST NOT remove these
                     "user-preference"]

  semantic:
    strategy: "importance"
    max_entries: 50000
    prune_threshold: 0.05
    consolidation_trigger: 1000        # Consolidate every N episodes

  graph:
    strategy: "reachability"
    orphan_ttl_hours: 168              # Orphan nodes: 7 days TTL
    max_depth: 50                      # Maximum graph depth

  versioning:
    max_snapshots: 100
    auto_snapshot_interval: "1h"
    compression: "zstd | gzip | lz4"
```

### 4.8 — Security and Access Control Schema

```yaml
security:
  # Per-layer access control
  access_control:
    episodic:
      read: ["agent", "auditor"]
      write: ["agent"]
      delete: ["admin"]
    semantic:
      read: ["agent", "peer-agent", "auditor"]
      write: ["agent", "admin"]
      delete: ["admin"]
    graph:
      read: ["agent", "peer-agent"]
      write: ["agent"]
      delete: ["admin"]
    procedural:
      read: ["agent", "peer-agent", "registry"]
      write: ["agent"]
      delete: ["admin"]

  # Encryption
  encryption:
    at_rest: "AES-256-GCM"             # MUST encrypt at rest
    in_transit: "TLS 1.3"              # MUST encrypt in transit
    key_management: "external"         # MUST use external KMS

  # Audit trail
  audit:
    log_memory_access: true            # MUST log memory operations
    log_skill_invocation: true         # MUST log skill usage
    log_model_calls: true              # SHOULD log model calls
    retention_days: 365                # MUST define retention
    format: "acp-audit-v1"            # MUST use standard format

  # Privacy
  privacy:
    pii_detection: true                # SHOULD detect PII
    pii_strategy: "redact |
                   encrypt |
                   exclude"
    gdpr_compliant: true               # MUST comply if applicable
    right_to_forget: true              # MUST support deletion requests
```

---

## 5 — Protocol Operations

ACP defines standardized operations that any compliant runtime MUST implement.

### 5.1 — Memory Operations

| Operation | Description | Required |
|-----------|-------------|----------|
| `acp.memory.store(layer, entry)` | Store an entry in a memory layer | MUST |
| `acp.memory.recall(query, layers[], top_k)` | Retrieve relevant entries | MUST |
| `acp.memory.forget(entry_id, strategy)` | Remove or decay an entry | MUST |
| `acp.memory.consolidate(source, target)` | Extract patterns from episodic → semantic | SHOULD |
| `acp.memory.prune(policy)` | Apply retention policy | MUST |
| `acp.memory.stats(layers[])` | Get memory statistics | SHOULD |

### 5.2 — Context Graph Operations

| Operation | Description | Required |
|-----------|-------------|----------|
| `acp.context.add_node(node)` | Add a node to the context graph | MUST |
| `acp.context.add_edge(edge)` | Add a typed edge between nodes | MUST |
| `acp.context.query(pattern)` | Query graph by pattern | MUST |
| `acp.context.subgraph(root_id, depth)` | Extract a subgraph | SHOULD |
| `acp.context.merge(external_graph)` | Merge an external graph | MAY |
| `acp.context.traverse(start, relation, depth)` | Walk the graph | SHOULD |

### 5.3 — Skill Operations

| Operation | Description | Required |
|-----------|-------------|----------|
| `acp.skill.register(skill)` | Register a new skill | MUST |
| `acp.skill.resolve(trigger_context)` | Find matching skills for context | MUST |
| `acp.skill.invoke(skill_id, params)` | Execute a skill | MUST |
| `acp.skill.update(skill_id, new_version)` | Update skill version | SHOULD |
| `acp.skill.export(skill_id)` | Export as portable object | SHOULD |
| `acp.skill.metrics(skill_id)` | Get performance metrics | MAY |

### 5.4 — Version Operations

| Operation | Description | Required |
|-----------|-------------|----------|
| `acp.version.snapshot(layers[])` | Create memory snapshot | MUST |
| `acp.version.restore(version_id)` | Restore previous state | MUST |
| `acp.version.diff(v1, v2)` | Compare two snapshots | SHOULD |
| `acp.version.branch(name)` | Create a memory branch | MAY |
| `acp.version.merge(branch)` | Merge a memory branch | MAY |
| `acp.version.list()` | List available snapshots | MUST |

### 5.5 — Exchange Operations

| Operation | Description | Required |
|-----------|-------------|----------|
| `acp.exchange.export(agent_id, format)` | Export agent state | MUST |
| `acp.exchange.import(artifact, target)` | Import agent state | MUST |
| `acp.exchange.share(layers[], recipient)` | Share specific layers | SHOULD |
| `acp.exchange.sync(remote_endpoint)` | Synchronize with remote | MAY |

---

## 6 — Wire Format

ACP uses **JSON-RPC 2.0** as its wire format, ensuring compatibility with MCP.

### 6.1 — Request Format

```json
{
  "jsonrpc": "2.0",
  "method": "acp.memory.recall",
  "params": {
    "query": "What is the return policy?",
    "layers": ["semantic", "episodic"],
    "top_k": 5,
    "min_confidence": 0.5,
    "include_provenance": true
  },
  "id": "req-001"
}
```

### 6.2 — Response Format

```json
{
  "jsonrpc": "2.0",
  "result": {
    "entries": [
      {
        "layer": "semantic",
        "id": "sem-042",
        "content": "Return policy is 30 days for electronics, 90 days for clothing",
        "confidence": 0.94,
        "relevance_score": 0.97,
        "provenance": {
          "source_episodes": ["ep-001", "ep-002", "ep-003"],
          "consolidation_id": "cons-007",
          "created_at": "2026-03-01T10:00:00Z",
          "verified": false
        }
      }
    ],
    "metadata": {
      "search_latency_ms": 12,
      "layers_searched": ["semantic", "episodic"],
      "total_candidates_scanned": 234,
      "results_returned": 1
    }
  },
  "id": "req-001"
}
```

### 6.3 — Error Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Layer not found",
    "data": {
      "requested_layer": "visual",
      "available_layers": ["episodic", "semantic", "graph", "procedural"]
    }
  },
  "id": "req-002"
}
```

### 6.4 — ACP Error Codes

| Code | Meaning |
|------|---------|
| -32001 | Layer not found |
| -32002 | Entry not found |
| -32003 | Version not found |
| -32004 | Skill not found |
| -32005 | Access denied |
| -32006 | Retention policy violation |
| -32007 | Embedding dimension mismatch |
| -32008 | Graph cycle detected |
| -32009 | Consolidation failed |
| -32010 | Snapshot limit exceeded |

### 6.5 — Transport

ACP MUST support at least one of:
- **stdio** (for local agents — pipes stdin/stdout)
- **HTTP + SSE** (for networked agents — server-sent events for streaming)
- **WebSocket** (for real-time bidirectional communication)

This matches MCP's transport model, enabling an ACP server to also serve as an MCP server.

---

## 7 — Interoperability

### 7.1 — ACP ↔ MCP

```
┌──────────────────────────────────────────────────────┐
│  MCP Tool Call                                        │
│  ┌─────────┐    ┌──────────┐    ┌──────────────┐    │
│  │ Agent   │───►│ MCP      │───►│ External     │    │
│  │         │◄───│ Server   │◄───│ Tool         │    │
│  └────┬────┘    └──────────┘    └──────────────┘    │
│       │                                              │
│       │  ACP automatically logs:                     │
│       │                                              │
│       ├── Episodic: tool invocation + result         │
│       ├── Semantic: extracted knowledge from result   │
│       ├── Graph: tool → task → result chain          │
│       └── Procedural: tool usage pattern learned      │
└──────────────────────────────────────────────────────┘
```

### 7.2 — ACP ↔ A2A

```
┌──────────────────────────────────────────────────────┐
│  Agent-to-Agent Communication                         │
│                                                      │
│  Agent A (ACP)              Agent B (ACP)            │
│  ┌──────────┐               ┌──────────┐            │
│  │ Context  │──── A2A ─────►│ Context  │            │
│  │ Graph    │    share      │ Graph    │            │
│  │          │   subgraph    │          │            │
│  │ Skills   │──── A2A ─────►│ Skills   │            │
│  │          │    delegate   │          │            │
│  └──────────┘               └──────────┘            │
│                                                      │
│  ACP enriches A2A Agent Cards with:                  │
│  - Memory capabilities declaration                   │
│  - Available shared skills                           │
│  - Context exchange format                           │
└──────────────────────────────────────────────────────┘
```

### 7.3 — ACP ↔ WebMCP

```
┌──────────────────────────────────────────────────────┐
│  Web Interaction                                      │
│                                                      │
│  ┌──────────┐    ┌──────────┐    ┌──────────────┐   │
│  │ Agent    │───►│ WebMCP   │───►│ Website      │   │
│  │ (ACP)   │◄───│ API      │◄───│              │   │
│  └────┬────┘    └──────────┘    └──────────────┘   │
│       │                                              │
│       │  ACP automatically captures:                 │
│       ├── Episodic: web interactions as episodes     │
│       ├── Semantic: extracted web content → knowledge │
│       └── Procedural: navigation patterns learned     │
└──────────────────────────────────────────────────────┘
```

### 7.4 — ACP ↔ NIST Compliance

```
┌──────────────────────────────────────────────────────┐
│  NIST AI Agent Standards Compliance                   │
│                                                      │
│  ACP provides:                                       │
│  ├── Agent identity (urn:acp:agent:<uuid>)           │
│  ├── Audit trail (all memory + skill operations)     │
│  ├── Provenance tracking (episodic → semantic chain) │
│  ├── Access control (per-layer RBAC)                 │
│  ├── Encryption (at rest + in transit)               │
│  └── Version history (complete cognitive timeline)    │
│                                                      │
│  Enables:                                            │
│  ├── "Why did the agent make this decision?" → trace │
│  ├── "What did the agent know at time T?" → restore  │
│  ├── "Who accessed the agent's memory?" → audit      │
│  └── "Can we reproduce this agent?" → snapshot       │
└──────────────────────────────────────────────────────┘
```

---

## 8 — Benchmark Suite

For scientific validation, ACP defines a benchmark suite measuring memory system quality.

### 8.1 — ACP-Recall (Memory Retrieval Quality)

| Metric | Description | Target |
|--------|-------------|--------|
| `recall@k` | Proportion of relevant entries in top-k results | > 0.85 |
| `precision@k` | Relevance of returned results | > 0.90 |
| `latency_p99` | 99th percentile retrieval time | < 100ms |
| `cross-layer` | Quality when searching across multiple layers | > 0.80 |

**Datasets**: `acp-bench/recall-100`, `acp-bench/recall-1000`, `acp-bench/recall-10000`

### 8.2 — ACP-Reconstruct (Context Recovery)

| Metric | Description | Target |
|--------|-------------|--------|
| `reconstruction_fidelity` | % of context correctly restored after crash | > 0.95 |
| `restart_latency` | Time to become operational from snapshot | < 5s |
| `token_efficiency` | Tokens needed vs raw context replay | < 0.3x |

### 8.3 — ACP-Consolidate (Knowledge Extraction)

| Metric | Description | Target |
|--------|-------------|--------|
| `compression_ratio` | Size reduction after consolidation | > 10x |
| `knowledge_preservation` | % of useful knowledge retained | > 0.90 |
| `false_generalization` | % of incorrect conclusions generated | < 0.05 |

### 8.4 — ACP-Portable (Cross-Model Transfer)

| Metric | Description | Target |
|--------|-------------|--------|
| `task_completion_delta` | Change in task completion rate after migration | < -0.10 |
| `skill_transfer_rate` | % of skills functional after model change | > 0.85 |
| `memory_compatibility` | % of memory usable by new model | > 0.95 |

### 8.5 — ACP-Scale (Scalability)

| Metric | Description | Target |
|--------|-------------|--------|
| `recall_latency_curve` | Latency vs memory size (should be sublinear) | O(log n) |
| `storage_efficiency` | Bytes per memory entry | < 1KB avg |
| `graph_query_time` | Graph traversal time vs node count | O(log n) |

---

## 9 — Academic Publication Plan

### 9.1 — Paper: "ACP: A Standard Protocol for Agent Memory and Context"

**Target venues:** NeurIPS 2026, ICML 2026, AAAI 2027, or ICLR 2027 Workshop on MemAgents

**Paper structure:**
1. **Abstract** — The memory standardization gap in agentic AI
2. **Introduction** — The protocol stack (MCP, A2A, WebMCP) and the missing layer
3. **Related Work** — Survey of agent memory approaches (A-MEM, MemAgents, framework-specific)
4. **ACP Specification** — The four-layer memory model, context graph, skill objects
5. **AGX Reference Implementation** — Architecture, format, runtime
6. **Evaluation** — Benchmark results (ACP-Recall, ACP-Reconstruct, ACP-Consolidate, ACP-Portable, ACP-Scale)
7. **Discussion** — Interoperability, adoption path, limitations
8. **Conclusion** — ACP as Layer 4 of the agentic protocol stack

### 9.2 — RFC: "ACP/1.0 — Agent Context Protocol Specification"

**Format:** IETF RFC-style document with:
- Numbered sections
- RFC 2119 keywords (MUST, SHOULD, MAY)
- Formal schemas (YAML/JSON Schema)
- Wire format specification (JSON-RPC 2.0)
- Error codes and semantics
- Security considerations
- IANA considerations

---

## 10 — Relationship with AGX

```
ACP = Protocol (the specification)
AGX = Implementation (the reference runtime + format)

ACP defines:
├── Memory model (4 layers)
├── Schemas (identity, episodic, semantic, graph, procedural, versioning)
├── Operations (store, recall, forget, consolidate, snapshot, ...)
├── Wire format (JSON-RPC 2.0)
├── Security model (RBAC, encryption, audit)
└── Benchmarks (recall, reconstruct, consolidate, portable, scale)

AGX implements:
├── .agx file format (OCI-compatible archive)
├── agxd daemon (runtime engine)
├── agx CLI (developer tool)
├── AGX Registry (distribution hub)
└── Shim layer (model adapters, MCP adapters)
```

Other runtimes MAY implement ACP independently. AGX is the reference — not the only — implementation.

---

## 11 — Glossary

| Term | Definition |
|------|-----------|
| **ACP** | Agent Context Protocol — the specification |
| **AGX** | Agent Graph eXchange — the reference implementation |
| **Episode** | A single event in episodic memory (conversation turn, action, observation) |
| **Consolidation** | The process of extracting semantic knowledge from episodic experiences |
| **Context Graph** | A directed graph representing an agent's working context and causal chains |
| **Skill Object** | A versioned, portable, measurable unit of agent capability |
| **Snapshot** | A content-addressable point-in-time capture of all memory layers |
| **Retention Policy** | Rules governing how and when an agent forgets |
| **Memory Branch** | A parallel copy of memory for exploratory reasoning |
| **Provenance** | The chain of evidence linking a knowledge entry to its source episodes |

---

## 12 — References

### Standards and Protocols
- [MCP — Model Context Protocol (Anthropic / Linux Foundation)](https://modelcontextprotocol.io/)
- [A2A — Agent-to-Agent Protocol (Google / Linux Foundation)](https://a2a-protocol.org/)
- [WebMCP — W3C Standard for Agent-Web Interaction](https://webmcp.link/)
- [OCI — Open Container Initiative](https://opencontainers.org/)
- [NIST AI Agent Standards Initiative (February 2026)](https://www.nist.gov/caisi/ai-agent-standards-initiative)

### Academic Research
- [Memory in the Age of AI Agents: A Survey (December 2025)](https://arxiv.org/abs/2512.13564)
- [A-MEM: Agentic Memory for LLM Agents (February 2025)](https://arxiv.org/abs/2502.12110)
- [AMA-Bench: Evaluating Long-Horizon Memory](https://arxiv.org/html/2602.22769v1)
- [ICLR 2026 Workshop: MemAgents](https://openreview.net/pdf?id=U51WxL382H)

### Ecosystem
- [AGX — Agent Graph eXchange (Reference Implementation)](../../AGX/docs/README.md)
- [Docker for AI Agents](https://www.docker.com/solutions/docker-ai/)
- [CNCF ModelPack Specification](https://www.cncf.io/blog/2025/08/27/how-oci-artifacts-will-drive-future-ai-use-cases/)

---

**Version:** 1.0.0-draft
**Date:** 2026-03-05
**Status:** Specification Draft
**License:** Apache 2.0