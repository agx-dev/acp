use async_trait::async_trait;
use rusqlite::params;

use acp_core::types::skill::*;
use acp_core::{AcpError, EntryId, SkillId, SkillRegistry};

use crate::store::SqliteStore;

/// Reconstruct a `SkillObject` from a SQLite row.
///
/// Column order must match:
///   0: id, 1: name, 2: version, 3: description, 4: instruction,
///   5: trigger_patterns, 6: context_conditions, 7: explicit_invocation,
///   8: tools_required, 9: skills_required, 10: min_context_window,
///   11: invocation_count, 12: success_rate, 13: avg_tokens_per_use, 14: avg_latency_ms,
///   15: last_used, 16: changelog,
///   17: created_at, 18: updated_at
fn row_to_skill(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillObject> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let version_str: String = row.get(2)?;
    let description: String = row.get(3)?;
    let instruction: String = row.get(4)?;

    let trigger_patterns_json: String = row.get(5)?;
    let context_conditions_json: String = row.get(6)?;
    let explicit_invocation: bool = row.get(7)?;

    let tools_required_json: String = row.get(8)?;
    let skills_required_json: String = row.get(9)?;
    let min_context_window: Option<u32> = row.get(10)?;

    let invocation_count: i64 = row.get(11)?;
    let success_rate: f64 = row.get(12)?;
    let avg_tokens_per_use: f64 = row.get(13)?;
    let avg_latency_ms: f64 = row.get(14)?;
    let last_used_str: Option<String> = row.get(15)?;

    let changelog_json: String = row.get(16)?;
    let created_at_str: String = row.get(17)?;
    let updated_at_str: String = row.get(18)?;

    // Parse version
    let version = semver::Version::parse(&version_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    // Parse JSON columns
    let patterns: Vec<TriggerPattern> =
        serde_json::from_str(&trigger_patterns_json).unwrap_or_default();
    let context_conditions: Vec<ContextCondition> =
        serde_json::from_str(&context_conditions_json).unwrap_or_default();
    let tools_required: Vec<String> =
        serde_json::from_str(&tools_required_json).unwrap_or_default();
    let skills_required: Vec<String> =
        serde_json::from_str(&skills_required_json).unwrap_or_default();
    let changelog: Vec<ChangelogEntry> =
        serde_json::from_str(&changelog_json).unwrap_or_default();

    // Parse timestamps
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let last_used = last_used_str.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok()
    });

    Ok(SkillObject {
        id: EntryId(id),
        name,
        version,
        description,
        instruction,
        trigger: SkillTrigger {
            patterns,
            context_conditions,
            explicit_invocation,
        },
        dependencies: SkillDependencies {
            tools_required,
            skills_required,
            min_context_window,
        },
        performance: SkillPerformance {
            invocation_count: invocation_count as u64,
            success_rate,
            avg_tokens_per_use,
            avg_latency_ms,
            last_used,
        },
        changelog,
        created_at,
        updated_at,
    })
}

#[async_trait]
impl SkillRegistry for SqliteStore {
    async fn register(&self, skill: SkillObject) -> Result<SkillId, AcpError> {
        self.store_skill(skill)
    }

    async fn get(&self, id: &SkillId) -> Result<SkillObject, AcpError> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, name, version, description, instruction,
                    trigger_patterns, context_conditions, explicit_invocation,
                    tools_required, skills_required, min_context_window,
                    invocation_count, success_rate, avg_tokens_per_use, avg_latency_ms,
                    last_used, changelog,
                    created_at, updated_at
             FROM skills WHERE id = ?1",
            params![id.0],
            row_to_skill,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AcpError::SkillNotFound(id.0.clone()),
            other => AcpError::Internal(other.to_string()),
        })
    }

    async fn list(&self) -> Result<Vec<SkillObject>, AcpError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, version, description, instruction,
                        trigger_patterns, context_conditions, explicit_invocation,
                        tools_required, skills_required, min_context_window,
                        invocation_count, success_rate, avg_tokens_per_use, avg_latency_ms,
                        last_used, changelog,
                        created_at, updated_at
                 FROM skills ORDER BY name",
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_skill)
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AcpError::Internal(e.to_string()))
    }

    async fn update(&self, id: &SkillId, skill: SkillObject) -> Result<(), AcpError> {
        let conn = self.conn();
        let rows_affected = conn
            .execute(
                "UPDATE skills SET
                    name = ?2, version = ?3, description = ?4, instruction = ?5,
                    trigger_patterns = ?6, context_conditions = ?7, explicit_invocation = ?8,
                    tools_required = ?9, skills_required = ?10, min_context_window = ?11,
                    invocation_count = ?12, success_rate = ?13, avg_tokens_per_use = ?14, avg_latency_ms = ?15,
                    last_used = ?16, changelog = ?17,
                    updated_at = ?18
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
                    skill.performance.invocation_count as i64,
                    skill.performance.success_rate,
                    skill.performance.avg_tokens_per_use,
                    skill.performance.avg_latency_ms,
                    skill.performance.last_used.map(|d| d.to_rfc3339()),
                    serde_json::to_string(&skill.changelog)
                        .map_err(|e| AcpError::Internal(e.to_string()))?,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        if rows_affected == 0 {
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

    async fn resolve(&self, context: &SkillContext) -> Result<Vec<SkillMatch>, AcpError> {
        // Get all skills, then score them against the context
        let all_skills = self.list().await?;
        let mut matches = Vec::new();

        for skill in all_skills {
            let mut score = 0.0;
            let mut reasons = Vec::new();

            // 1. Check trigger patterns (regex match on query)
            for pattern in &skill.trigger.patterns {
                if let Ok(re) = regex::Regex::new(&pattern.regex) {
                    if re.is_match(&context.query) {
                        score += pattern.confidence_threshold;
                        reasons.push(format!("pattern '{}' matched", pattern.regex));
                    }
                }
            }

            // 2. Check if skill name/description matches query (simple word matching)
            let query_lower = context.query.to_lowercase();
            let name_lower = skill.name.to_lowercase();
            let desc_lower = skill.description.to_lowercase();
            if name_lower.contains(&query_lower) || query_lower.contains(&name_lower) {
                score += 0.5;
                reasons.push("name match".into());
            }
            if desc_lower.contains(&query_lower) {
                score += 0.3;
                reasons.push("description match".into());
            }

            // 3. Check tool availability
            if !skill.dependencies.tools_required.is_empty() {
                let available = skill
                    .dependencies
                    .tools_required
                    .iter()
                    .all(|t| context.available_tools.contains(t));
                if available {
                    score += 0.2;
                    reasons.push("all required tools available".into());
                } else {
                    score -= 0.5;
                    reasons.push("missing required tools".into());
                }
            }

            if score > 0.0 {
                matches.push(SkillMatch {
                    skill,
                    confidence: score.min(1.0),
                    match_reason: reasons.join("; "),
                });
            }
        }

        // Sort by confidence descending
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        Ok(matches)
    }
}
