use serde::{Deserialize, Serialize};

/// All ACP protocol methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcpMethod {
    // Memory operations (Core)
    MemoryStore,
    MemoryRecall,
    MemoryForget,
    MemoryPrune,
    MemoryStats,

    // Graph operations (Standard)
    GraphAddNode,
    GraphAddEdge,
    GraphQuery,
    GraphSubgraph,
    GraphTraverse,
    GraphRemoveNode,
    GraphRemoveEdge,

    // Skill operations (Full)
    SkillRegister,
    SkillResolve,
    SkillGet,
    SkillUpdate,
    SkillExport,
    SkillList,

    // Version operations (Standard)
    VersionSnapshot,
    VersionRestore,
    VersionDiff,
    VersionList,

    // Exchange operations (Core)
    ExchangeExport,
    ExchangeImport,
}

impl AcpMethod {
    /// Parse a method string into an AcpMethod.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "acp.memory.store" => Some(Self::MemoryStore),
            "acp.memory.recall" => Some(Self::MemoryRecall),
            "acp.memory.forget" => Some(Self::MemoryForget),
            "acp.memory.prune" => Some(Self::MemoryPrune),
            "acp.memory.stats" => Some(Self::MemoryStats),

            "acp.graph.add_node" => Some(Self::GraphAddNode),
            "acp.graph.add_edge" => Some(Self::GraphAddEdge),
            "acp.graph.query" => Some(Self::GraphQuery),
            "acp.graph.subgraph" => Some(Self::GraphSubgraph),
            "acp.graph.traverse" => Some(Self::GraphTraverse),
            "acp.graph.remove_node" => Some(Self::GraphRemoveNode),
            "acp.graph.remove_edge" => Some(Self::GraphRemoveEdge),

            "acp.skill.register" => Some(Self::SkillRegister),
            "acp.skill.resolve" => Some(Self::SkillResolve),
            "acp.skill.get" => Some(Self::SkillGet),
            "acp.skill.update" => Some(Self::SkillUpdate),
            "acp.skill.export" => Some(Self::SkillExport),
            "acp.skill.list" => Some(Self::SkillList),

            "acp.version.snapshot" => Some(Self::VersionSnapshot),
            "acp.version.restore" => Some(Self::VersionRestore),
            "acp.version.diff" => Some(Self::VersionDiff),
            "acp.version.list" => Some(Self::VersionList),

            "acp.exchange.export" => Some(Self::ExchangeExport),
            "acp.exchange.import" => Some(Self::ExchangeImport),

            _ => None,
        }
    }

    /// The wire name for this method.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MemoryStore => "acp.memory.store",
            Self::MemoryRecall => "acp.memory.recall",
            Self::MemoryForget => "acp.memory.forget",
            Self::MemoryPrune => "acp.memory.prune",
            Self::MemoryStats => "acp.memory.stats",

            Self::GraphAddNode => "acp.graph.add_node",
            Self::GraphAddEdge => "acp.graph.add_edge",
            Self::GraphQuery => "acp.graph.query",
            Self::GraphSubgraph => "acp.graph.subgraph",
            Self::GraphTraverse => "acp.graph.traverse",
            Self::GraphRemoveNode => "acp.graph.remove_node",
            Self::GraphRemoveEdge => "acp.graph.remove_edge",

            Self::SkillRegister => "acp.skill.register",
            Self::SkillResolve => "acp.skill.resolve",
            Self::SkillGet => "acp.skill.get",
            Self::SkillUpdate => "acp.skill.update",
            Self::SkillExport => "acp.skill.export",
            Self::SkillList => "acp.skill.list",

            Self::VersionSnapshot => "acp.version.snapshot",
            Self::VersionRestore => "acp.version.restore",
            Self::VersionDiff => "acp.version.diff",
            Self::VersionList => "acp.version.list",

            Self::ExchangeExport => "acp.exchange.export",
            Self::ExchangeImport => "acp.exchange.import",
        }
    }

    /// Minimum conformance level required for this method.
    pub fn conformance(&self) -> crate::types::ConformanceLevel {
        use crate::types::ConformanceLevel::*;
        match self {
            Self::MemoryStore
            | Self::MemoryRecall
            | Self::MemoryForget
            | Self::MemoryPrune
            | Self::MemoryStats
            | Self::ExchangeExport
            | Self::ExchangeImport => Core,

            Self::GraphAddNode
            | Self::GraphAddEdge
            | Self::GraphQuery
            | Self::GraphSubgraph
            | Self::GraphTraverse
            | Self::GraphRemoveNode
            | Self::GraphRemoveEdge
            | Self::VersionSnapshot
            | Self::VersionRestore
            | Self::VersionDiff
            | Self::VersionList => Standard,

            Self::SkillRegister
            | Self::SkillResolve
            | Self::SkillGet
            | Self::SkillUpdate
            | Self::SkillExport
            | Self::SkillList => Full,
        }
    }
}

impl std::fmt::Display for AcpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_method() {
        let method = AcpMethod::MemoryRecall;
        let s = method.as_str();
        let parsed = AcpMethod::parse(s).unwrap();
        assert_eq!(method, parsed);
    }

    #[test]
    fn unknown_method_returns_none() {
        assert!(AcpMethod::parse("acp.foo.bar").is_none());
    }
}
