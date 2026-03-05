use serde::{Deserialize, Serialize};

use crate::types::{ConformanceLevel, RetentionPolicy};

/// ACP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpConfig {
    /// ACP protocol version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Conformance level.
    #[serde(default)]
    pub conformance: ConformanceLevel,
    /// Storage path.
    pub storage_path: Option<String>,
    /// Retention policies.
    #[serde(default)]
    pub retention: RetentionPolicy,
    /// Embedding configuration.
    pub embeddings: Option<EmbeddingConfig>,
    /// Auto-record episodes.
    #[serde(default = "default_true")]
    pub auto_record: bool,
    /// Project scope detection.
    #[serde(default)]
    pub scope: ScopeConfig,
}

fn default_version() -> String {
    "0.1.0".into()
}

fn default_true() -> bool {
    true
}

impl Default for AcpConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            conformance: ConformanceLevel::Core,
            storage_path: None,
            retention: RetentionPolicy::default(),
            embeddings: None,
            auto_record: true,
            scope: ScopeConfig::default(),
        }
    }
}

impl Default for ConformanceLevel {
    fn default() -> Self {
        Self::Core
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: Option<String>,
    pub dimensions: Option<usize>,
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeConfig {
    pub mode: ScopeMode,
    pub path: Option<String>,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            mode: ScopeMode::Auto,
            path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeMode {
    Auto,
    Git,
    Directory,
    Manual,
}
