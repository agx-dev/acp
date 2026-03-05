use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::{Confidence, EntryId};

/// A single episode in episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: EntryId,
    pub seq_num: u64,
    pub timestamp: DateTime<Utc>,
    pub episode_type: EpisodeType,
    pub content: EpisodeContent,
    pub context: EpisodeContext,
    pub outcome: Option<Outcome>,
    pub metadata: EpisodeMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeType {
    Conversation,
    Action,
    Observation,
    Reflection,
    Error,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeContent {
    pub role: Role,
    pub text: String,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_output: Option<serde_json::Value>,
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Agent,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeContext {
    pub session_id: String,
    pub conversation_id: Option<String>,
    pub parent_episode: Option<EntryId>,
    pub graph_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub status: OutcomeStatus,
    pub confidence: Option<Confidence>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeStatus {
    Success,
    Failure,
    Partial,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeMetadata {
    #[serde(default)]
    pub importance: Option<f64>,
    #[serde(default)]
    pub trigger: Option<Trigger>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub model_used: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    UserInput,
    ToolResponse,
    Scheduled,
    Event,
    Consolidation,
    Reflection,
}
