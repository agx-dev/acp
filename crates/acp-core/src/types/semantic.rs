use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::{Confidence, EntryId};

/// A semantic memory entry — structured knowledge derived from episodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEntry {
    pub id: EntryId,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub source: SemanticSource,
    pub confidence: Confidence,
    pub importance: f64,
    pub access_count: u64,
    pub last_accessed: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub domain: Option<String>,
    #[serde(default)]
    pub protected: bool,
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f64,
    pub provenance: Option<Provenance>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_decay_rate() -> f64 {
    0.01
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SemanticSource {
    Consolidated,
    External,
    Manual,
    Inferred,
    Peer,
}

/// Tracks how a semantic entry was derived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source_episodes: Vec<EntryId>,
    pub consolidation_id: Option<String>,
    #[serde(default)]
    pub verified: bool,
    pub verification_date: Option<DateTime<Utc>>,
}
