use async_trait::async_trait;

use crate::types::*;
use crate::AcpError;

/// Trait for skill management (Conformance: Full).
#[async_trait]
pub trait SkillRegistry: Send + Sync {
    /// Register a new skill.
    async fn register(&self, skill: SkillObject) -> Result<SkillId, AcpError>;

    /// Resolve matching skills for a given context.
    async fn resolve(&self, context: &SkillContext) -> Result<Vec<SkillMatch>, AcpError>;

    /// Get a skill by ID.
    async fn get(&self, id: &SkillId) -> Result<SkillObject, AcpError>;

    /// Update an existing skill.
    async fn update(&self, id: &SkillId, skill: SkillObject) -> Result<(), AcpError>;

    /// Export a skill for sharing.
    async fn export(&self, id: &SkillId) -> Result<PortableSkill, AcpError>;

    /// List all registered skills.
    async fn list(&self) -> Result<Vec<SkillObject>, AcpError>;

    /// Invoke a skill by ID: returns instruction + input, increments invocation_count.
    /// `input` is optional context data (pass `Value::Null` or an empty object if unused).
    async fn invoke(
        &self,
        id: &SkillId,
        input: serde_json::Value,
    ) -> Result<SkillInvocationResult, AcpError>;
}
