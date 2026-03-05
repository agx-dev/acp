use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::EntryId;

/// A skill — a reusable procedural routine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillObject {
    pub id: EntryId,
    pub name: String,
    pub version: semver::Version,
    pub description: String,
    pub instruction: String,
    pub trigger: SkillTrigger,
    pub dependencies: SkillDependencies,
    pub performance: SkillPerformance,
    #[serde(default)]
    pub changelog: Vec<ChangelogEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrigger {
    #[serde(default)]
    pub patterns: Vec<TriggerPattern>,
    #[serde(default)]
    pub context_conditions: Vec<ContextCondition>,
    #[serde(default)]
    pub explicit_invocation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPattern {
    pub regex: String,
    #[serde(default = "default_threshold")]
    pub confidence_threshold: f64,
}

fn default_threshold() -> f64 {
    0.7
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCondition {
    pub key: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
    Exists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependencies {
    #[serde(default)]
    pub tools_required: Vec<String>,
    #[serde(default)]
    pub skills_required: Vec<String>,
    pub min_context_window: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPerformance {
    #[serde(default)]
    pub invocation_count: u64,
    #[serde(default)]
    pub success_rate: f64,
    #[serde(default)]
    pub avg_tokens_per_use: f64,
    #[serde(default)]
    pub avg_latency_ms: f64,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub version: semver::Version,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Result of skill resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMatch {
    pub skill: SkillObject,
    pub confidence: f64,
    pub match_reason: String,
}

/// Context passed to skill resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub query: String,
    pub available_tools: Vec<String>,
    pub session_tags: Vec<String>,
}

/// A skill packaged for sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableSkill {
    pub skill: SkillObject,
    pub source_agent: Option<String>,
    pub exported_at: DateTime<Utc>,
}
