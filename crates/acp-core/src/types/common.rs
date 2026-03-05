use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AcpError;

/// Unique identifier for any entry in the ACP system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub String);

impl EntryId {
    pub fn new(prefix: &str) -> Self {
        Self(format!("{}-{}", prefix, Uuid::new_v4()))
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type aliases for clarity.
pub type NodeId = EntryId;
pub type EdgeId = EntryId;
pub type SkillId = EntryId;

/// ACP memory layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Episodic,
    Semantic,
    Graph,
    Procedural,
}

/// ACP conformance levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConformanceLevel {
    Core,
    Standard,
    Full,
}

/// Confidence score bounded to [0.0, 1.0].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Confidence(f64);

impl Confidence {
    pub fn new(value: f64) -> Result<Self, AcpError> {
        if !(0.0..=1.0).contains(&value) {
            return Err(AcpError::InvalidConfidence(value));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl PartialEq for Confidence {
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0).abs() < f64::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_id_generates_unique() {
        let a = EntryId::new("ep");
        let b = EntryId::new("ep");
        assert_ne!(a, b);
        assert!(a.0.starts_with("ep-"));
    }

    #[test]
    fn confidence_validates_range() {
        assert!(Confidence::new(0.5).is_ok());
        assert!(Confidence::new(0.0).is_ok());
        assert!(Confidence::new(1.0).is_ok());
        assert!(Confidence::new(-0.1).is_err());
        assert!(Confidence::new(1.1).is_err());
    }

    #[test]
    fn layer_serializes_lowercase() {
        let json = serde_json::to_string(&Layer::Episodic).unwrap();
        assert_eq!(json, "\"episodic\"");
    }
}
