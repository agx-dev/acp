use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::ConformanceLevel;

/// Agent identity — who this agent is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub agent_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub conformance: ConformanceLevel,
    pub public_key: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AgentIdentity {
    pub fn new(name: impl Into<String>, conformance: ConformanceLevel) -> Self {
        Self {
            agent_id: format!("agent-{}", uuid::Uuid::new_v4()),
            name: name.into(),
            version: "0.1.0".into(),
            description: None,
            conformance,
            public_key: None,
            parent_id: None,
            created_at: Utc::now(),
        }
    }
}
