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
    MemoryConsolidate,

    // Context graph operations (Standard). Wire namespace is `acp.context.*`
    // per the specification; `acp.graph.*` is accepted as a legacy alias.
    GraphAddNode,
    GraphAddEdge,
    GraphQuery,
    GraphSubgraph,
    GraphTraverse,
    GraphMerge,
    GraphRemoveNode,
    GraphRemoveEdge,

    // Skill operations (Full)
    SkillRegister,
    SkillResolve,
    SkillGet,
    SkillUpdate,
    SkillExport,
    SkillList,
    SkillInvoke,

    // Version operations (Standard)
    VersionSnapshot,
    VersionRestore,
    VersionDiff,
    VersionList,
    VersionBranch,
    VersionMerge,

    // Exchange operations (Core)
    ExchangeExport,
    ExchangeImport,
    ExchangeShare,

    // Spec / introspection (Core)
    Capabilities,
    Health,
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
            "acp.memory.consolidate" => Some(Self::MemoryConsolidate),

            // Canonical `acp.context.*` namespace (spec)
            "acp.context.add_node" => Some(Self::GraphAddNode),
            "acp.context.add_edge" => Some(Self::GraphAddEdge),
            "acp.context.query" => Some(Self::GraphQuery),
            "acp.context.subgraph" => Some(Self::GraphSubgraph),
            "acp.context.traverse" => Some(Self::GraphTraverse),
            "acp.context.merge" => Some(Self::GraphMerge),
            "acp.context.remove_node" => Some(Self::GraphRemoveNode),
            "acp.context.remove_edge" => Some(Self::GraphRemoveEdge),

            // Legacy `acp.graph.*` aliases (accepted, not advertised)
            "acp.graph.add_node" => Some(Self::GraphAddNode),
            "acp.graph.add_edge" => Some(Self::GraphAddEdge),
            "acp.graph.query" => Some(Self::GraphQuery),
            "acp.graph.subgraph" => Some(Self::GraphSubgraph),
            "acp.graph.traverse" => Some(Self::GraphTraverse),
            "acp.graph.merge" => Some(Self::GraphMerge),
            "acp.graph.remove_node" => Some(Self::GraphRemoveNode),
            "acp.graph.remove_edge" => Some(Self::GraphRemoveEdge),

            "acp.skill.register" => Some(Self::SkillRegister),
            "acp.skill.resolve" => Some(Self::SkillResolve),
            "acp.skill.get" => Some(Self::SkillGet),
            "acp.skill.update" => Some(Self::SkillUpdate),
            "acp.skill.export" => Some(Self::SkillExport),
            "acp.skill.list" => Some(Self::SkillList),
            "acp.skill.invoke" => Some(Self::SkillInvoke),

            "acp.version.snapshot" => Some(Self::VersionSnapshot),
            "acp.version.restore" => Some(Self::VersionRestore),
            "acp.version.diff" => Some(Self::VersionDiff),
            "acp.version.list" => Some(Self::VersionList),
            "acp.version.branch" => Some(Self::VersionBranch),
            "acp.version.merge" => Some(Self::VersionMerge),

            "acp.exchange.export" => Some(Self::ExchangeExport),
            "acp.exchange.import" => Some(Self::ExchangeImport),
            "acp.exchange.share" => Some(Self::ExchangeShare),

            "acp.capabilities" => Some(Self::Capabilities),
            "acp.health" => Some(Self::Health),

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
            Self::MemoryConsolidate => "acp.memory.consolidate",

            Self::GraphAddNode => "acp.context.add_node",
            Self::GraphAddEdge => "acp.context.add_edge",
            Self::GraphQuery => "acp.context.query",
            Self::GraphSubgraph => "acp.context.subgraph",
            Self::GraphTraverse => "acp.context.traverse",
            Self::GraphMerge => "acp.context.merge",
            Self::GraphRemoveNode => "acp.context.remove_node",
            Self::GraphRemoveEdge => "acp.context.remove_edge",

            Self::SkillRegister => "acp.skill.register",
            Self::SkillResolve => "acp.skill.resolve",
            Self::SkillGet => "acp.skill.get",
            Self::SkillUpdate => "acp.skill.update",
            Self::SkillExport => "acp.skill.export",
            Self::SkillList => "acp.skill.list",
            Self::SkillInvoke => "acp.skill.invoke",

            Self::VersionSnapshot => "acp.version.snapshot",
            Self::VersionRestore => "acp.version.restore",
            Self::VersionDiff => "acp.version.diff",
            Self::VersionList => "acp.version.list",
            Self::VersionBranch => "acp.version.branch",
            Self::VersionMerge => "acp.version.merge",

            Self::ExchangeExport => "acp.exchange.export",
            Self::ExchangeImport => "acp.exchange.import",
            Self::ExchangeShare => "acp.exchange.share",

            Self::Capabilities => "acp.capabilities",
            Self::Health => "acp.health",
        }
    }

    /// All protocol methods in declaration order.
    pub fn all() -> &'static [Self] {
        use AcpMethod::*;
        &[
            MemoryStore, MemoryRecall, MemoryForget, MemoryPrune, MemoryStats, MemoryConsolidate,
            GraphAddNode, GraphAddEdge, GraphQuery, GraphSubgraph, GraphTraverse, GraphMerge,
            GraphRemoveNode, GraphRemoveEdge,
            SkillRegister, SkillResolve, SkillGet, SkillUpdate, SkillExport, SkillList,
            SkillInvoke,
            VersionSnapshot, VersionRestore, VersionDiff, VersionList, VersionBranch, VersionMerge,
            ExchangeExport, ExchangeImport, ExchangeShare,
            Capabilities, Health,
        ]
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
            | Self::ExchangeImport
            | Self::Capabilities
            | Self::Health => Core,

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
            | Self::VersionList
            | Self::ExchangeShare
            | Self::MemoryConsolidate => Standard,

            Self::SkillRegister
            | Self::SkillResolve
            | Self::SkillGet
            | Self::SkillUpdate
            | Self::SkillExport
            | Self::SkillList
            | Self::SkillInvoke
            | Self::GraphMerge
            | Self::VersionBranch
            | Self::VersionMerge => Full,
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

    #[test]
    fn new_methods_parse_and_roundtrip() {
        for (s, expected) in [
            ("acp.capabilities", AcpMethod::Capabilities),
            ("acp.health", AcpMethod::Health),
            ("acp.skill.invoke", AcpMethod::SkillInvoke),
        ] {
            let m = AcpMethod::parse(s).expect(s);
            assert_eq!(m, expected);
            assert_eq!(m.as_str(), s);
        }
    }

    #[test]
    fn capabilities_and_health_are_core() {
        use crate::types::ConformanceLevel::*;
        assert_eq!(AcpMethod::Capabilities.conformance(), Core);
        assert_eq!(AcpMethod::Health.conformance(), Core);
        assert_eq!(AcpMethod::SkillInvoke.conformance(), Full);
    }

    #[test]
    fn context_is_canonical_graph_is_alias() {
        // The spec wire namespace is `acp.context.*`; `acp.graph.*` is a legacy alias.
        assert_eq!(AcpMethod::GraphAddNode.as_str(), "acp.context.add_node");
        assert_eq!(AcpMethod::GraphMerge.as_str(), "acp.context.merge");

        // Both namespaces parse to the same variant.
        assert_eq!(
            AcpMethod::parse("acp.context.add_node"),
            Some(AcpMethod::GraphAddNode)
        );
        assert_eq!(
            AcpMethod::parse("acp.graph.add_node"),
            Some(AcpMethod::GraphAddNode)
        );

        // Canonical name round-trips through as_str -> parse.
        for m in AcpMethod::all() {
            assert_eq!(AcpMethod::parse(m.as_str()), Some(*m), "roundtrip {}", m);
        }
    }

    #[test]
    fn newly_wired_full_methods_parse() {
        use crate::types::ConformanceLevel::*;
        for (s, expected, level) in [
            ("acp.context.merge", AcpMethod::GraphMerge, Full),
            ("acp.version.branch", AcpMethod::VersionBranch, Full),
            ("acp.version.merge", AcpMethod::VersionMerge, Full),
            ("acp.exchange.share", AcpMethod::ExchangeShare, Standard),
        ] {
            let m = AcpMethod::parse(s).expect(s);
            assert_eq!(m, expected);
            assert_eq!(m.as_str(), s);
            assert_eq!(m.conformance(), level, "conformance {}", s);
        }
    }
}
