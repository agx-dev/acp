//! # ACP Core — Agent Context Protocol
//!
//! Pure-logic crate defining all ACP types, traits, and wire format.
//! No IO, no network, no filesystem — just data structures and contracts.

pub mod config;
pub mod ops;
pub mod protocol;
pub mod types;

// Re-export the most commonly used items at the crate root.
pub use config::AcpConfig;
pub use protocol::AcpError;

// Re-export all types.
pub use types::*;

// Re-export all ops traits.
pub use ops::{
    AgentBundle, ContextGraphStore, Exchange, MemoryStats, MemoryStore, RecallEntry, RecallQuery,
    RecallResult, SkillRegistry, StoreEntry, VersionManager,
};

// Re-export protocol items.
pub use protocol::{AcpMethod, JsonRpcError, JsonRpcRequest, JsonRpcResponse};
