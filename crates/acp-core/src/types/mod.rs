pub mod agent;
pub mod common;
pub mod episode;
pub mod graph;
pub mod retention;
pub mod semantic;
pub mod skill;
pub mod version;

// Re-export key types at the types level.
pub use agent::AgentIdentity;
pub use common::{Confidence, ConformanceLevel, EdgeId, EntryId, Layer, NodeId, SkillId};
pub use episode::{
    Episode, EpisodeContent, EpisodeContext, EpisodeMetadata, EpisodeType, Outcome, OutcomeStatus,
    Role, Trigger,
};
pub use graph::{Edge, GraphPattern, Node, NodeType, Relation, SubGraph};
pub use retention::{EvictionStrategy, ForgetStrategy, PruneReport, RetentionPolicy};
pub use semantic::{Provenance, SemanticEntry, SemanticSource};
pub use skill::{PortableSkill, SkillContext, SkillMatch, SkillObject};
pub use version::{SnapshotConfig, SnapshotInfo, VersionDiff};
