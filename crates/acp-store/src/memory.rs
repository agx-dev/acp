use async_trait::async_trait;
use rusqlite::params;

use acp_core::ops::memory::LayerStats;
use acp_core::types::retention::ForgetStrategy;
use acp_core::{
    AcpError, EntryId, Episode, Layer, MemoryStats, MemoryStore, RecallEntry, RecallQuery,
    RecallResult, SemanticEntry, SkillObject, StoreEntry,
};

use crate::store::SqliteStore;

/// Convert a serde-serializable enum to its lowercase SQL string representation.
fn enum_to_sql<T: serde::Serialize>(val: &T) -> Result<String, AcpError> {
    let json = serde_json::to_value(val).map_err(|e| AcpError::Internal(e.to_string()))?;
    Ok(json.as_str().unwrap_or_default().to_string())
}

/// Escape a user query string for safe use in FTS5 MATCH.
/// Wraps each word in double quotes to prevent FTS5 operator interpretation.
fn fts5_escape(query: &str) -> String {
    query
        .split_whitespace()
        .map(|word| {
            let escaped = word.replace('"', "\"\"");
            format!("\"{}\"", escaped)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[async_trait]
impl MemoryStore for SqliteStore {
    async fn store(&self, layer: Layer, entry: StoreEntry) -> Result<EntryId, AcpError> {
        match (layer, entry) {
            (Layer::Episodic, StoreEntry::Episode(ep)) => self.store_episode(ep),
            (Layer::Semantic, StoreEntry::Semantic(se)) => self.store_semantic(se),
            (Layer::Procedural, StoreEntry::Skill(sk)) => self.store_skill(sk),
            _ => Err(AcpError::LayerNotFound(layer)),
        }
    }

    async fn recall(&self, query: RecallQuery) -> Result<RecallResult, AcpError> {
        let start = std::time::Instant::now();
        let mut entries = Vec::new();

        let layers = if query.layers.is_empty() {
            vec![Layer::Episodic, Layer::Semantic, Layer::Procedural]
        } else {
            query.layers.clone()
        };

        for layer in &layers {
            match layer {
                Layer::Episodic => entries.extend(self.recall_episodes(&query)?),
                Layer::Semantic => entries.extend(self.recall_semantic(&query)?),
                Layer::Procedural => entries.extend(self.recall_skills(&query)?),
                Layer::Graph => {}
            }
        }

        entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let top_k = query.top_k.unwrap_or(10);
        entries.truncate(top_k);

        let total_count = entries.len() as u64;
        let query_time_ms = start.elapsed().as_millis() as u64;

        Ok(RecallResult {
            entries,
            total_count,
            query_time_ms,
        })
    }

    async fn forget(&self, id: &EntryId, strategy: ForgetStrategy) -> Result<(), AcpError> {
        let conn = self.conn();

        // Check if protected
        let is_protected: bool = conn
            .query_row(
                "SELECT COALESCE(
                    (SELECT protected FROM semantic_entries WHERE id = ?1),
                    (SELECT protected FROM episodes WHERE id = ?1),
                    0
                )",
                params![id.0],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if is_protected {
            return Err(AcpError::ProtectedEntry(id.0.clone()));
        }

        match strategy {
            ForgetStrategy::Hard => {
                conn.execute("DELETE FROM episodes WHERE id = ?1", params![id.0])
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
                conn.execute("DELETE FROM semantic_entries WHERE id = ?1", params![id.0])
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
                conn.execute("DELETE FROM skills WHERE id = ?1", params![id.0])
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
            }
            ForgetStrategy::Soft => {
                conn.execute(
                    "UPDATE episodes SET deleted_at = datetime('now') WHERE id = ?1",
                    params![id.0],
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
                conn.execute(
                    "UPDATE semantic_entries SET deleted_at = datetime('now') WHERE id = ?1",
                    params![id.0],
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            }
            ForgetStrategy::Redact => {
                conn.execute(
                    "UPDATE episodes SET content_text = '[REDACTED]', tool_input = NULL, tool_output = NULL WHERE id = ?1",
                    params![id.0],
                ).map_err(|e| AcpError::Internal(e.to_string()))?;
                conn.execute(
                    "UPDATE semantic_entries SET content = '[REDACTED]', embedding = NULL WHERE id = ?1",
                    params![id.0],
                ).map_err(|e| AcpError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn prune(
        &self,
        policy: &acp_core::types::retention::RetentionPolicy,
    ) -> Result<acp_core::types::retention::PruneReport, AcpError> {
        let conn = self.conn();
        let mut report = acp_core::types::retention::PruneReport::default();

        // Prune old episodes
        if let Some(max_age) = policy.episodic.max_age_days {
            let deleted: usize = conn
                .execute(
                    "DELETE FROM episodes WHERE deleted_at IS NULL AND protected = 0
                     AND timestamp < datetime('now', ?1)",
                    params![format!("-{} days", max_age)],
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            report.episodes_pruned = deleted as u64;
        }

        // Prune by max count
        if let Some(max_eps) = policy.episodic.max_episodes {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM episodes WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;

            if count > max_eps as i64 {
                let to_remove = count - max_eps as i64;
                let deleted: usize = conn
                    .execute(
                        "DELETE FROM episodes WHERE id IN (
                            SELECT id FROM episodes WHERE deleted_at IS NULL AND protected = 0
                            ORDER BY timestamp ASC LIMIT ?1
                        )",
                        params![to_remove],
                    )
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
                report.episodes_pruned += deleted as u64;
            }
        }

        // Prune low-importance semantic entries
        if let Some(min_imp) = policy.semantic.min_importance {
            let deleted: usize = conn
                .execute(
                    "DELETE FROM semantic_entries WHERE deleted_at IS NULL AND protected = 0
                     AND importance < ?1",
                    params![min_imp],
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            report.semantic_pruned = deleted as u64;
        }

        // Prune orphan graph nodes
        if policy.graph.prune_orphans {
            let deleted: usize = conn
                .execute(
                    "DELETE FROM nodes WHERE id NOT IN (
                        SELECT source FROM edges UNION SELECT target FROM edges
                    )",
                    [],
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            report.nodes_pruned = deleted as u64;
        }

        Ok(report)
    }

    async fn stats(&self, layers: &[Layer]) -> Result<MemoryStats, AcpError> {
        let conn = self.conn();
        let mut stats = MemoryStats::default();

        let layers = if layers.is_empty() {
            vec![Layer::Episodic, Layer::Semantic, Layer::Procedural, Layer::Graph]
        } else {
            layers.to_vec()
        };

        for layer in &layers {
            match layer {
                Layer::Episodic => {
                    let count: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM episodes WHERE deleted_at IS NULL",
                            [],
                            |row| row.get(0),
                        )
                        .map_err(|e| AcpError::Internal(e.to_string()))?;
                    stats.episodes_count = count as u64;
                    stats.layer_stats.push(LayerStats {
                        layer: Layer::Episodic,
                        entry_count: count as u64,
                        size_bytes: 0,
                    });
                }
                Layer::Semantic => {
                    let count: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM semantic_entries WHERE deleted_at IS NULL",
                            [],
                            |row| row.get(0),
                        )
                        .map_err(|e| AcpError::Internal(e.to_string()))?;
                    stats.semantic_count = count as u64;
                    stats.layer_stats.push(LayerStats {
                        layer: Layer::Semantic,
                        entry_count: count as u64,
                        size_bytes: 0,
                    });
                }
                Layer::Procedural => {
                    let count: i64 = conn
                        .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
                        .map_err(|e| AcpError::Internal(e.to_string()))?;
                    stats.skills_count = count as u64;
                    stats.layer_stats.push(LayerStats {
                        layer: Layer::Procedural,
                        entry_count: count as u64,
                        size_bytes: 0,
                    });
                }
                Layer::Graph => {}
            }
        }

        Ok(stats)
    }
}

// --- Private implementation methods ---

impl SqliteStore {
    fn store_episode(&self, ep: Episode) -> Result<EntryId, AcpError> {
        let id = EntryId::new("ep");
        let conn = self.conn();

        conn.execute(
            "INSERT INTO episodes (
                id, seq_num, timestamp, episode_type,
                role, content_text, tool_name, tool_input, tool_output,
                tokens_input, tokens_output,
                session_id, conversation_id, parent_episode, graph_ref,
                outcome_status, outcome_confidence, outcome_error_code,
                importance, trigger_type, tags, model_used, latency_ms
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8, ?9,
                ?10, ?11,
                ?12, ?13, ?14, ?15,
                ?16, ?17, ?18,
                ?19, ?20, ?21, ?22, ?23
            )",
            params![
                id.0,
                ep.seq_num,
                ep.timestamp.to_rfc3339(),
                enum_to_sql(&ep.episode_type)?,
                enum_to_sql(&ep.content.role)?,
                ep.content.text,
                ep.content.tool_name,
                ep.content.tool_input.as_ref().map(|v| v.to_string()),
                ep.content.tool_output.as_ref().map(|v| v.to_string()),
                ep.content.tokens_input,
                ep.content.tokens_output,
                ep.context.session_id,
                ep.context.conversation_id,
                ep.context.parent_episode.as_ref().map(|e| &e.0),
                ep.context.graph_ref,
                ep.outcome
                    .as_ref()
                    .map(|o| enum_to_sql(&o.status).unwrap()),
                ep.outcome
                    .as_ref()
                    .and_then(|o| o.confidence.map(|c| c.value())),
                ep.outcome.as_ref().and_then(|o| o.error_code.clone()),
                ep.metadata.importance.unwrap_or(0.5),
                ep.metadata
                    .trigger
                    .as_ref()
                    .map(|t| serde_json::to_string(t).unwrap()),
                serde_json::to_string(&ep.metadata.tags)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                ep.metadata.model_used,
                ep.metadata.latency_ms,
            ],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(id)
    }

    fn store_semantic(&self, se: SemanticEntry) -> Result<EntryId, AcpError> {
        let id = EntryId::new("sem");
        let conn = self.conn();

        let embedding_blob: Option<Vec<u8>> = se.embedding.as_ref().map(|emb| {
            emb.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect()
        });

        conn.execute(
            "INSERT INTO semantic_entries (
                id, content, embedding, source, confidence, importance, decay_rate,
                access_count, last_accessed, tags, category, domain,
                protected, source_episodes, consolidation_id,
                verified, verification_date
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17
            )",
            params![
                id.0,
                se.content,
                embedding_blob,
                enum_to_sql(&se.source)?,
                se.confidence.value(),
                se.importance,
                se.decay_rate,
                se.access_count,
                se.last_accessed.map(|dt| dt.to_rfc3339()),
                serde_json::to_string(&se.tags)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                se.category,
                se.domain,
                se.protected,
                se.provenance
                    .as_ref()
                    .map(|p| serde_json::to_string(&p.source_episodes).unwrap())
                    .unwrap_or_else(|| "[]".to_string()),
                se.provenance.as_ref().and_then(|p| p.consolidation_id.clone()),
                se.provenance.as_ref().map(|p| p.verified).unwrap_or(false),
                se.provenance
                    .as_ref()
                    .and_then(|p| p.verification_date.map(|d| d.to_rfc3339())),
            ],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(id)
    }

    pub(crate) fn store_skill(&self, sk: SkillObject) -> Result<EntryId, AcpError> {
        let id = EntryId::new("skill");
        let conn = self.conn();

        conn.execute(
            "INSERT INTO skills (
                id, name, version, description, instruction,
                trigger_patterns, context_conditions, explicit_invocation,
                tools_required, skills_required, min_context_window,
                invocation_count, success_rate, avg_tokens_per_use, avg_latency_ms,
                last_used, changelog
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8,
                ?9, ?10, ?11,
                ?12, ?13, ?14, ?15,
                ?16, ?17
            )",
            params![
                id.0,
                sk.name,
                sk.version.to_string(),
                sk.description,
                sk.instruction,
                serde_json::to_string(&sk.trigger.patterns)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                serde_json::to_string(&sk.trigger.context_conditions)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                sk.trigger.explicit_invocation,
                serde_json::to_string(&sk.dependencies.tools_required)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                serde_json::to_string(&sk.dependencies.skills_required)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
                sk.dependencies.min_context_window,
                sk.performance.invocation_count,
                sk.performance.success_rate,
                sk.performance.avg_tokens_per_use,
                sk.performance.avg_latency_ms,
                sk.performance.last_used.map(|d| d.to_rfc3339()),
                serde_json::to_string(&sk.changelog)
                    .map_err(|e| AcpError::Internal(e.to_string()))?,
            ],
        )
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(id)
    }

    fn recall_episodes(&self, query: &RecallQuery) -> Result<Vec<RecallEntry>, AcpError> {
        let conn = self.conn();

        let (sql, text_param) = if let Some(ref text) = query.text {
            (
                "SELECT e.id, e.content_text, e.importance, e.timestamp
                 FROM episodes e
                 JOIN episodes_fts fts ON fts.rowid = e.rowid
                 WHERE episodes_fts MATCH ?1
                 AND e.deleted_at IS NULL
                 ORDER BY rank
                 LIMIT ?2",
                Some(fts5_escape(text)),
            )
        } else {
            (
                "SELECT id, content_text, importance, timestamp
                 FROM episodes
                 WHERE deleted_at IS NULL
                 ORDER BY timestamp DESC
                 LIMIT ?1",
                None,
            )
        };

        let top_k = query.top_k.unwrap_or(10) as i64;
        let mut stmt = conn.prepare(sql).map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = if let Some(ref text) = text_param {
            stmt.query_map(params![text, top_k], map_episode_row)
        } else {
            stmt.query_map(params![top_k], map_episode_row)
        }
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }

    fn recall_semantic(&self, query: &RecallQuery) -> Result<Vec<RecallEntry>, AcpError> {
        let conn = self.conn();

        let (sql, text_param) = if let Some(ref text) = query.text {
            (
                "SELECT se.id, se.content, se.importance, se.confidence
                 FROM semantic_entries se
                 JOIN semantic_fts fts ON fts.rowid = se.rowid
                 WHERE semantic_fts MATCH ?1
                 AND se.deleted_at IS NULL
                 ORDER BY rank
                 LIMIT ?2",
                Some(fts5_escape(text)),
            )
        } else {
            (
                "SELECT id, content, importance, confidence
                 FROM semantic_entries
                 WHERE deleted_at IS NULL
                 ORDER BY importance DESC
                 LIMIT ?1",
                None,
            )
        };

        let top_k = query.top_k.unwrap_or(10) as i64;
        let mut stmt = conn.prepare(sql).map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = if let Some(ref text) = text_param {
            stmt.query_map(params![text, top_k], map_semantic_row)
        } else {
            stmt.query_map(params![top_k], map_semantic_row)
        }
        .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }

    fn recall_skills(&self, query: &RecallQuery) -> Result<Vec<RecallEntry>, AcpError> {
        let conn = self.conn();

        let top_k = query.top_k.unwrap_or(10) as i64;

        if let Some(ref text) = query.text {
            let escaped = fts5_escape(text);
            let mut s = conn
                .prepare(
                    "SELECT sk.id, sk.name || ': ' || sk.description, sk.success_rate
                     FROM skills sk
                     JOIN skills_fts fts ON fts.rowid = sk.rowid
                     WHERE skills_fts MATCH ?1
                     ORDER BY rank
                     LIMIT ?2",
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let rows = s
                .query_map(params![escaped, top_k], |row| {
                    Ok(RecallEntry {
                        id: EntryId(row.get(0)?),
                        layer: Layer::Procedural,
                        content: row.get(1)?,
                        score: row.get::<_, f64>(2).unwrap_or(0.5),
                        tags: vec![],
                        metadata: None,
                    })
                })
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AcpError::Internal(e.to_string()))
        } else {
            let mut s = conn
                .prepare(
                    "SELECT id, name || ': ' || description, success_rate
                     FROM skills
                     ORDER BY success_rate DESC
                     LIMIT ?1",
                )
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            let rows = s
                .query_map(params![top_k], |row| {
                    Ok(RecallEntry {
                        id: EntryId(row.get(0)?),
                        layer: Layer::Procedural,
                        content: row.get(1)?,
                        score: row.get::<_, f64>(2).unwrap_or(0.5),
                        tags: vec![],
                        metadata: None,
                    })
                })
                .map_err(|e| AcpError::Internal(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AcpError::Internal(e.to_string()))
        }
    }
}

fn map_episode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecallEntry> {
    Ok(RecallEntry {
        id: EntryId(row.get(0)?),
        layer: Layer::Episodic,
        content: row.get(1)?,
        score: row.get::<_, f64>(2).unwrap_or(0.5),
        tags: vec![],
        metadata: None,
    })
}

fn map_semantic_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecallEntry> {
    let importance: f64 = row.get::<_, f64>(2).unwrap_or(0.5);
    let confidence: f64 = row.get::<_, f64>(3).unwrap_or(0.5);
    Ok(RecallEntry {
        id: EntryId(row.get(0)?),
        layer: Layer::Semantic,
        content: row.get(1)?,
        score: importance * confidence,
        tags: vec![],
        metadata: None,
    })
}
