use rusqlite::params;

use acp_core::types::episode::*;
use acp_core::types::semantic::*;
use acp_core::{AcpError, Confidence, EntryId, SemanticEntry};

use crate::store::SqliteStore;

impl SqliteStore {
    /// Export all active episodes.
    pub fn export_all_episodes(&self) -> Result<Vec<Episode>, AcpError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, seq_num, timestamp, episode_type, role, content_text,
                        tool_name, tool_input, tool_output, tokens_input, tokens_output,
                        session_id, conversation_id, parent_episode, graph_ref,
                        outcome_status, outcome_confidence, outcome_error_code,
                        importance, trigger_type, tags, model_used, latency_ms
                 FROM episodes WHERE deleted_at IS NULL",
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let episode_type_str: String = row.get(3)?;
                let role_str: String = row.get(4)?;
                let timestamp_str: String = row.get(2)?;
                let tags_json: String = row.get(20)?;

                let episode_type = match episode_type_str.as_str() {
                    "conversation" => EpisodeType::Conversation,
                    "action" => EpisodeType::Action,
                    "observation" => EpisodeType::Observation,
                    "reflection" => EpisodeType::Reflection,
                    "error" => EpisodeType::Error,
                    _ => EpisodeType::System,
                };

                let role = match role_str.as_str() {
                    "user" => Role::User,
                    "agent" => Role::Agent,
                    "system" => Role::System,
                    _ => Role::Tool,
                };

                let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(Episode {
                    id: EntryId(row.get(0)?),
                    seq_num: row.get::<_, i64>(1)? as u64,
                    timestamp,
                    episode_type,
                    content: EpisodeContent {
                        role,
                        text: row.get(5)?,
                        tool_name: row.get(6)?,
                        tool_input: row
                            .get::<_, Option<String>>(7)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        tool_output: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        tokens_input: row.get::<_, Option<i32>>(9)?.map(|v| v as u32),
                        tokens_output: row.get::<_, Option<i32>>(10)?.map(|v| v as u32),
                    },
                    context: EpisodeContext {
                        session_id: row.get(11)?,
                        conversation_id: row.get(12)?,
                        parent_episode: row.get::<_, Option<String>>(13)?.map(EntryId),
                        graph_ref: row.get(14)?,
                    },
                    outcome: row.get::<_, Option<String>>(15)?.map(|status_str| {
                        let status = match status_str.as_str() {
                            "success" => OutcomeStatus::Success,
                            "failure" => OutcomeStatus::Failure,
                            "partial" => OutcomeStatus::Partial,
                            _ => OutcomeStatus::Pending,
                        };
                        Outcome {
                            status,
                            confidence: row
                                .get::<_, Option<f64>>(16)
                                .ok()
                                .flatten()
                                .and_then(|v| Confidence::new(v).ok()),
                            error_code: row.get(17).ok().flatten(),
                        }
                    }),
                    metadata: EpisodeMetadata {
                        importance: row.get::<_, Option<f64>>(18).ok().flatten(),
                        trigger: None,
                        tags,
                        model_used: row.get(21).ok().flatten(),
                        latency_ms: row.get::<_, Option<i64>>(22).ok().flatten().map(|v| v as u64),
                    },
                })
            })
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }

    /// Export all active semantic entries.
    pub fn export_all_semantic(&self) -> Result<Vec<SemanticEntry>, AcpError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, content, source, confidence, importance, decay_rate,
                        access_count, last_accessed, tags, category, domain, protected,
                        created_at, updated_at
                 FROM semantic_entries WHERE deleted_at IS NULL",
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let source_str: String = row.get(2)?;
                let tags_json: String = row.get(8)?;
                let created_str: String = row.get(12)?;
                let updated_str: String = row.get(13)?;

                let source = match source_str.as_str() {
                    "manual" => SemanticSource::Manual,
                    "consolidated" => SemanticSource::Consolidated,
                    "inferred" => SemanticSource::Inferred,
                    _ => SemanticSource::Manual,
                };

                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                Ok(SemanticEntry {
                    id: EntryId(row.get(0)?),
                    content: row.get(1)?,
                    embedding: None,
                    source,
                    confidence: Confidence::new(row.get::<_, f64>(3).unwrap_or(0.5))
                        .unwrap_or_else(|_| Confidence::new(0.5).unwrap()),
                    importance: row.get(4)?,
                    decay_rate: row.get(5)?,
                    access_count: row.get::<_, i64>(6)? as u64,
                    last_accessed: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .ok()
                        }),
                    tags,
                    category: row.get(9)?,
                    domain: row.get(10)?,
                    protected: row.get(11)?,
                    provenance: None,
                    created_at,
                    updated_at,
                })
            })
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }

    /// Import episodes (inserts, ignores conflicts).
    pub fn import_episodes(&self, episodes: &[Episode]) -> Result<u64, AcpError> {
        let conn = self.conn();
        let mut count = 0u64;
        for ep in episodes {
            let result = conn.execute(
                "INSERT OR IGNORE INTO episodes (
                    id, seq_num, timestamp, episode_type, role, content_text,
                    tool_name, session_id, importance, tags
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    ep.id.0,
                    ep.seq_num as i64,
                    ep.timestamp.to_rfc3339(),
                    serde_json::to_value(&ep.episode_type)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "system".into()),
                    serde_json::to_value(&ep.content.role)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "agent".into()),
                    ep.content.text,
                    ep.content.tool_name,
                    ep.context.session_id,
                    ep.metadata.importance.unwrap_or(0.5),
                    serde_json::to_string(&ep.metadata.tags).unwrap_or_else(|_| "[]".into()),
                ],
            );
            if let Ok(n) = result {
                count += n as u64;
            }
        }
        Ok(count)
    }

    /// Import semantic entries (inserts, ignores conflicts).
    pub fn import_semantic(&self, entries: &[SemanticEntry]) -> Result<u64, AcpError> {
        let conn = self.conn();
        let mut count = 0u64;
        for se in entries {
            let result = conn.execute(
                "INSERT OR IGNORE INTO semantic_entries (
                    id, content, source, confidence, importance, decay_rate,
                    access_count, tags, category, domain, protected
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    se.id.0,
                    se.content,
                    serde_json::to_value(&se.source)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "manual".into()),
                    se.confidence.value(),
                    se.importance,
                    se.decay_rate,
                    se.access_count as i64,
                    serde_json::to_string(&se.tags).unwrap_or_else(|_| "[]".into()),
                    se.category,
                    se.domain,
                    se.protected,
                ],
            );
            if let Ok(n) = result {
                count += n as u64;
            }
        }
        Ok(count)
    }
}
