//! # ACP Store — SQLite Storage Backend
//!
//! Implements `acp-core` traits using embedded SQLite.
//! Zero external dependencies — SQLite is bundled at compile time.

mod exchange;
mod graph;
mod memory;
mod schema;
mod skills;
mod store;
mod versioning;

pub use store::{SqliteStore, StoreConfig};

/// Store-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum AcpStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("ACP error: {0}")]
    Acp(#[from] acp_core::AcpError),
}

// Allow AcpStoreError to convert into AcpError for trait implementations.
impl From<AcpStoreError> for acp_core::AcpError {
    fn from(e: AcpStoreError) -> Self {
        acp_core::AcpError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use acp_core::ops::memory::StoreEntry;
    use acp_core::types::episode::*;
    use acp_core::types::semantic::*;
    use acp_core::*;

    use super::*;

    fn make_episode(text: &str, session: &str) -> Episode {
        Episode {
            id: EntryId::new("ep"),
            seq_num: 1,
            timestamp: chrono::Utc::now(),
            episode_type: EpisodeType::Conversation,
            content: EpisodeContent {
                role: Role::User,
                text: text.to_string(),
                tool_name: None,
                tool_input: None,
                tool_output: None,
                tokens_input: None,
                tokens_output: None,
            },
            context: EpisodeContext {
                session_id: session.to_string(),
                conversation_id: None,
                parent_episode: None,
                graph_ref: None,
            },
            outcome: None,
            metadata: EpisodeMetadata {
                importance: Some(0.7),
                trigger: None,
                tags: vec!["test".to_string()],
                model_used: None,
                latency_ms: None,
            },
        }
    }

    fn make_semantic(content: &str, importance: f64) -> SemanticEntry {
        SemanticEntry {
            id: EntryId::new("sem"),
            content: content.to_string(),
            embedding: None,
            source: SemanticSource::Manual,
            confidence: Confidence::new(0.9).unwrap(),
            importance,
            access_count: 0,
            last_accessed: None,
            tags: vec![],
            category: None,
            domain: None,
            protected: false,
            decay_rate: 0.01,
            provenance: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_store_and_recall_episode() {
        let store = SqliteStore::in_memory().unwrap();
        let ep = make_episode("How does authentication work?", "sess-1");

        let id = store
            .store(Layer::Episodic, StoreEntry::Episode(ep))
            .await
            .unwrap();
        assert!(id.0.starts_with("ep-"));

        let result = store
            .recall(RecallQuery {
                text: Some("authentication".to_string()),
                layers: vec![Layer::Episodic],
                top_k: Some(5),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].content.contains("authentication"));
    }

    #[tokio::test]
    async fn test_store_and_recall_semantic() {
        let store = SqliteStore::in_memory().unwrap();
        let se = make_semantic("This project uses hexagonal architecture", 0.8);

        let id = store
            .store(Layer::Semantic, StoreEntry::Semantic(se))
            .await
            .unwrap();
        assert!(id.0.starts_with("sem-"));

        let result = store
            .recall(RecallQuery {
                text: Some("hexagonal architecture".to_string()),
                layers: vec![Layer::Semantic],
                top_k: Some(5),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].content.contains("hexagonal"));
    }

    #[tokio::test]
    async fn test_forget_hard() {
        let store = SqliteStore::in_memory().unwrap();
        let ep = make_episode("Temporary data", "sess-1");

        let id = store
            .store(Layer::Episodic, StoreEntry::Episode(ep))
            .await
            .unwrap();

        store
            .forget(&id, acp_core::types::retention::ForgetStrategy::Hard)
            .await
            .unwrap();

        let stats = store.stats(&[Layer::Episodic]).await.unwrap();
        assert_eq!(stats.episodes_count, 0);
    }

    #[tokio::test]
    async fn test_forget_protected_entry_fails() {
        let store = SqliteStore::in_memory().unwrap();
        let mut se = make_semantic("Critical knowledge", 0.9);
        se.protected = true;

        let id = store
            .store(Layer::Semantic, StoreEntry::Semantic(se))
            .await
            .unwrap();

        let result = store
            .forget(&id, acp_core::types::retention::ForgetStrategy::Hard)
            .await;

        assert!(matches!(result, Err(AcpError::ProtectedEntry(_))));
    }

    #[tokio::test]
    async fn test_stats() {
        let store = SqliteStore::in_memory().unwrap();

        store
            .store(
                Layer::Episodic,
                StoreEntry::Episode(make_episode("ep1", "s1")),
            )
            .await
            .unwrap();
        store
            .store(
                Layer::Episodic,
                StoreEntry::Episode(make_episode("ep2", "s1")),
            )
            .await
            .unwrap();
        store
            .store(
                Layer::Semantic,
                StoreEntry::Semantic(make_semantic("sem1", 0.5)),
            )
            .await
            .unwrap();

        let stats = store
            .stats(&[Layer::Episodic, Layer::Semantic])
            .await
            .unwrap();
        assert_eq!(stats.episodes_count, 2);
        assert_eq!(stats.semantic_count, 1);
    }

    #[tokio::test]
    async fn test_store_and_recall_skill() {
        use acp_core::types::skill::*;

        let store = SqliteStore::in_memory().unwrap();
        let skill = SkillObject {
            id: EntryId::new("skill"),
            name: "git-commit".to_string(),
            version: semver::Version::new(1, 0, 0),
            description: "Create well-formatted git commits".to_string(),
            instruction: "Run git add then git commit with a descriptive message".to_string(),
            trigger: SkillTrigger {
                patterns: vec![],
                context_conditions: vec![],
                explicit_invocation: true,
            },
            dependencies: SkillDependencies {
                tools_required: vec!["bash".to_string()],
                skills_required: vec![],
                min_context_window: None,
            },
            performance: SkillPerformance {
                invocation_count: 0,
                success_rate: 0.0,
                avg_tokens_per_use: 0.0,
                avg_latency_ms: 0.0,
                last_used: None,
            },
            changelog: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = store
            .store(Layer::Procedural, StoreEntry::Skill(skill))
            .await
            .unwrap();
        assert!(id.0.starts_with("skill-"));

        let result = store
            .recall(RecallQuery {
                text: Some("git commit".to_string()),
                layers: vec![Layer::Procedural],
                top_k: Some(5),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].content.contains("git-commit"));
    }

    #[tokio::test]
    async fn test_recall_without_text_returns_recent() {
        let store = SqliteStore::in_memory().unwrap();

        store
            .store(
                Layer::Episodic,
                StoreEntry::Episode(make_episode("first message", "s1")),
            )
            .await
            .unwrap();
        store
            .store(
                Layer::Episodic,
                StoreEntry::Episode(make_episode("second message", "s1")),
            )
            .await
            .unwrap();

        let result = store
            .recall(RecallQuery {
                layers: vec![Layer::Episodic],
                top_k: Some(10),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 2);
    }

    #[tokio::test]
    async fn test_skill_register_and_get() {
        use acp_core::types::skill::*;
        use acp_core::SkillRegistry;

        let store = SqliteStore::in_memory().unwrap();
        let skill = SkillObject {
            id: EntryId::new("skill"),
            name: "code-review".to_string(),
            version: semver::Version::new(1, 0, 0),
            description: "Review code for quality issues".to_string(),
            instruction: "Analyze the diff and provide feedback".to_string(),
            trigger: SkillTrigger {
                patterns: vec![TriggerPattern {
                    regex: r"review|check".to_string(),
                    confidence_threshold: 0.7,
                }],
                context_conditions: vec![],
                explicit_invocation: false,
            },
            dependencies: SkillDependencies {
                tools_required: vec!["bash".to_string()],
                skills_required: vec![],
                min_context_window: None,
            },
            performance: SkillPerformance {
                invocation_count: 0,
                success_rate: 0.0,
                avg_tokens_per_use: 0.0,
                avg_latency_ms: 0.0,
                last_used: None,
            },
            changelog: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = store.register(skill).await.unwrap();
        assert!(id.0.starts_with("skill-"));

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.name, "code-review");
        assert_eq!(retrieved.description, "Review code for quality issues");
        assert_eq!(retrieved.dependencies.tools_required, vec!["bash"]);
    }

    #[tokio::test]
    async fn test_skill_list() {
        use acp_core::types::skill::*;
        use acp_core::SkillRegistry;

        let store = SqliteStore::in_memory().unwrap();
        let make = |name: &str| SkillObject {
            id: EntryId::new("skill"),
            name: name.to_string(),
            version: semver::Version::new(1, 0, 0),
            description: format!("{} desc", name),
            instruction: "do thing".to_string(),
            trigger: SkillTrigger {
                patterns: vec![],
                context_conditions: vec![],
                explicit_invocation: true,
            },
            dependencies: SkillDependencies {
                tools_required: vec![],
                skills_required: vec![],
                min_context_window: None,
            },
            performance: SkillPerformance {
                invocation_count: 0,
                success_rate: 0.0,
                avg_tokens_per_use: 0.0,
                avg_latency_ms: 0.0,
                last_used: None,
            },
            changelog: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        store.register(make("skill-b")).await.unwrap();
        store.register(make("skill-a")).await.unwrap();

        let all = SkillRegistry::list(&store).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "skill-a"); // ORDER BY name
        assert_eq!(all[1].name, "skill-b");
    }

    #[tokio::test]
    async fn test_skill_update() {
        use acp_core::types::skill::*;
        use acp_core::SkillRegistry;

        let store = SqliteStore::in_memory().unwrap();
        let skill = SkillObject {
            id: EntryId::new("skill"),
            name: "old-name".to_string(),
            version: semver::Version::new(1, 0, 0),
            description: "old desc".to_string(),
            instruction: "old instruction".to_string(),
            trigger: SkillTrigger {
                patterns: vec![],
                context_conditions: vec![],
                explicit_invocation: true,
            },
            dependencies: SkillDependencies {
                tools_required: vec![],
                skills_required: vec![],
                min_context_window: None,
            },
            performance: SkillPerformance {
                invocation_count: 0,
                success_rate: 0.0,
                avg_tokens_per_use: 0.0,
                avg_latency_ms: 0.0,
                last_used: None,
            },
            changelog: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = store.register(skill).await.unwrap();
        let mut updated = store.get(&id).await.unwrap();
        updated.name = "new-name".to_string();
        updated.description = "new desc".to_string();
        updated.version = semver::Version::new(2, 0, 0);

        store.update(&id, updated).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.name, "new-name");
        assert_eq!(retrieved.description, "new desc");
        assert_eq!(retrieved.version, semver::Version::new(2, 0, 0));
    }

    #[tokio::test]
    async fn test_skill_export() {
        use acp_core::types::skill::*;
        use acp_core::SkillRegistry;

        let store = SqliteStore::in_memory().unwrap();
        let skill = SkillObject {
            id: EntryId::new("skill"),
            name: "exportable".to_string(),
            version: semver::Version::new(1, 0, 0),
            description: "A skill".to_string(),
            instruction: "do it".to_string(),
            trigger: SkillTrigger {
                patterns: vec![],
                context_conditions: vec![],
                explicit_invocation: true,
            },
            dependencies: SkillDependencies {
                tools_required: vec![],
                skills_required: vec![],
                min_context_window: None,
            },
            performance: SkillPerformance {
                invocation_count: 5,
                success_rate: 0.9,
                avg_tokens_per_use: 100.0,
                avg_latency_ms: 50.0,
                last_used: None,
            },
            changelog: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = store.register(skill).await.unwrap();
        let portable = store.export(&id).await.unwrap();
        assert_eq!(portable.skill.name, "exportable");
        assert!(portable.source_agent.is_none());
    }
}
