use rusqlite::Connection;

/// Apply the full ACP schema to a SQLite connection.
pub fn apply_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(SCHEMA_SQL)
}

const SCHEMA_SQL: &str = r#"
-- Pragmas
PRAGMA foreign_keys = ON;

-- =========================================================================
-- 1. EPISODIC MEMORY
-- =========================================================================

CREATE TABLE IF NOT EXISTS episodes (
    id              TEXT PRIMARY KEY,
    seq_num         INTEGER NOT NULL,
    timestamp       TEXT NOT NULL,
    episode_type    TEXT NOT NULL CHECK (
        episode_type IN ('conversation', 'action', 'observation',
                         'reflection', 'error', 'system')
    ),
    role            TEXT NOT NULL CHECK (role IN ('user', 'agent', 'system', 'tool')),
    content_text    TEXT NOT NULL,
    tool_name       TEXT,
    tool_input      TEXT,
    tool_output     TEXT,
    tokens_input    INTEGER,
    tokens_output   INTEGER,
    session_id      TEXT NOT NULL,
    conversation_id TEXT,
    parent_episode  TEXT,
    graph_ref       TEXT,
    outcome_status  TEXT CHECK (
        outcome_status IS NULL OR outcome_status IN ('success', 'failure', 'partial', 'pending')
    ),
    outcome_confidence  REAL CHECK (
        outcome_confidence IS NULL OR (outcome_confidence >= 0.0 AND outcome_confidence <= 1.0)
    ),
    outcome_error_code  TEXT,
    importance      REAL DEFAULT 0.5,
    trigger_type    TEXT,
    tags            TEXT DEFAULT '[]',
    model_used      TEXT,
    latency_ms      INTEGER,
    consolidated    INTEGER DEFAULT 0,
    protected       INTEGER DEFAULT 0,
    deleted_at      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_episodes_timestamp ON episodes(timestamp);
CREATE INDEX IF NOT EXISTS idx_episodes_session ON episodes(session_id);
CREATE INDEX IF NOT EXISTS idx_episodes_type ON episodes(episode_type);
CREATE INDEX IF NOT EXISTS idx_episodes_deleted ON episodes(deleted_at) WHERE deleted_at IS NULL;

-- FTS for episodes
CREATE VIRTUAL TABLE IF NOT EXISTS episodes_fts USING fts5(
    content_text,
    tool_name,
    tags,
    content='episodes',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS episodes_fts_insert AFTER INSERT ON episodes BEGIN
    INSERT INTO episodes_fts(rowid, content_text, tool_name, tags)
    VALUES (new.rowid, new.content_text, new.tool_name, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS episodes_fts_delete AFTER DELETE ON episodes BEGIN
    INSERT INTO episodes_fts(episodes_fts, rowid, content_text, tool_name, tags)
    VALUES ('delete', old.rowid, old.content_text, old.tool_name, old.tags);
END;

-- =========================================================================
-- 2. SEMANTIC MEMORY
-- =========================================================================

CREATE TABLE IF NOT EXISTS semantic_entries (
    id              TEXT PRIMARY KEY,
    content         TEXT NOT NULL,
    embedding       BLOB,
    source          TEXT NOT NULL CHECK (source IN (
        'consolidated', 'external', 'manual', 'inferred', 'peer'
    )),
    confidence      REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    importance      REAL NOT NULL DEFAULT 0.5,
    decay_rate      REAL NOT NULL DEFAULT 0.01,
    access_count    INTEGER NOT NULL DEFAULT 0,
    last_accessed   TEXT,
    tags            TEXT DEFAULT '[]',
    category        TEXT,
    domain          TEXT,
    protected       INTEGER DEFAULT 0,
    source_episodes TEXT DEFAULT '[]',
    consolidation_id TEXT,
    verified        INTEGER DEFAULT 0,
    verification_date TEXT,
    deleted_at      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_semantic_importance ON semantic_entries(importance DESC);
CREATE INDEX IF NOT EXISTS idx_semantic_category ON semantic_entries(category);
CREATE INDEX IF NOT EXISTS idx_semantic_domain ON semantic_entries(domain);
CREATE INDEX IF NOT EXISTS idx_semantic_deleted ON semantic_entries(deleted_at) WHERE deleted_at IS NULL;

-- FTS for semantic entries
CREATE VIRTUAL TABLE IF NOT EXISTS semantic_fts USING fts5(
    content,
    tags,
    category,
    domain,
    content='semantic_entries',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS semantic_fts_insert AFTER INSERT ON semantic_entries BEGIN
    INSERT INTO semantic_fts(rowid, content, tags, category, domain)
    VALUES (new.rowid, new.content, new.tags, new.category, new.domain);
END;

CREATE TRIGGER IF NOT EXISTS semantic_fts_delete AFTER DELETE ON semantic_entries BEGIN
    INSERT INTO semantic_fts(semantic_fts, rowid, content, tags, category, domain)
    VALUES ('delete', old.rowid, old.content, old.tags, old.category, old.domain);
END;

-- =========================================================================
-- 3. CONTEXT GRAPH
-- =========================================================================

CREATE TABLE IF NOT EXISTS nodes (
    id              TEXT PRIMARY KEY,
    node_type       TEXT NOT NULL CHECK (node_type IN (
        'task', 'decision', 'tool', 'result', 'knowledge',
        'entity', 'goal', 'constraint', 'event', 'artifact'
    )),
    label           TEXT NOT NULL,
    properties      TEXT DEFAULT '{}',
    embedding       BLOB,
    episode_refs    TEXT DEFAULT '[]',
    semantic_refs   TEXT DEFAULT '[]',
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);

CREATE TABLE IF NOT EXISTS edges (
    id              TEXT PRIMARY KEY,
    source          TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    target          TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    relation        TEXT NOT NULL CHECK (relation IN (
        'caused_by', 'led_to', 'triggered', 'part_of', 'contains',
        'depends_on', 'blocked_by', 'supports', 'contradicts',
        'refined_by', 'used_for', 'created_by', 'modified_by', 'resolved_by'
    )),
    weight          REAL NOT NULL DEFAULT 1.0,
    confidence      REAL,
    evidence        TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source);
CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target);
CREATE INDEX IF NOT EXISTS idx_edges_relation ON edges(relation);

-- =========================================================================
-- 4. SKILLS
-- =========================================================================

CREATE TABLE IF NOT EXISTS skills (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    version         TEXT NOT NULL,
    description     TEXT NOT NULL,
    instruction     TEXT NOT NULL,
    trigger_patterns    TEXT DEFAULT '[]',
    context_conditions  TEXT DEFAULT '[]',
    explicit_invocation INTEGER DEFAULT 1,
    tools_required      TEXT DEFAULT '[]',
    skills_required     TEXT DEFAULT '[]',
    min_context_window  INTEGER,
    invocation_count    INTEGER DEFAULT 0,
    success_rate        REAL DEFAULT 0.0,
    avg_tokens_per_use  REAL DEFAULT 0.0,
    avg_latency_ms      REAL DEFAULT 0.0,
    last_used           TEXT,
    changelog           TEXT DEFAULT '[]',
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

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

-- =========================================================================
-- 5. SNAPSHOTS
-- =========================================================================

CREATE TABLE IF NOT EXISTS snapshots (
    id              TEXT PRIMARY KEY,
    version         INTEGER NOT NULL UNIQUE,
    hash            TEXT NOT NULL,
    data            BLOB NOT NULL,
    reason          TEXT,
    size_bytes      INTEGER NOT NULL,
    compressed_bytes INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- =========================================================================
-- 6. METADATA
-- =========================================================================

CREATE TABLE IF NOT EXISTS metadata (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO metadata (key, value) VALUES
    ('schema_version', '1'),
    ('acp_version', '0.1.0');
"#;
