use serde_json::json;

/// Return the MCP tool definitions exposed by the ACP server.
pub fn mcp_tools() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "acp_recall",
            "description": "Search persistent agent memory. Returns relevant memorized knowledge for a query.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search text"
                    },
                    "layers": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["episodic", "semantic", "procedural", "graph"] },
                        "description": "Memory layers to search",
                        "default": ["semantic", "procedural"]
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 5
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "acp_store",
            "description": "Memorize important knowledge. Use to save conventions, patterns, architecture, and preferences discovered during work.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Content to memorize"
                    },
                    "layer": {
                        "type": "string",
                        "enum": ["semantic", "episodic"],
                        "description": "Memory layer to store in",
                        "default": "semantic"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tags for categorization"
                    },
                    "importance": {
                        "type": "number",
                        "description": "Importance score (0.0-1.0)",
                        "default": 0.7
                    },
                    "role": {
                        "type": "string",
                        "enum": ["user", "agent", "system", "tool"],
                        "description": "Role for episodic entries",
                        "default": "agent"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID for episodic entries"
                    }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "acp_context",
            "description": "Query the context graph to see relationships between project components.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "root": {
                        "type": "string",
                        "description": "Root node ID"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Traversal depth",
                        "default": 2
                    }
                },
                "required": ["root"]
            }
        }),
        json!({
            "name": "acp_graph_traverse",
            "description": "Traverse the context graph following a specific relation type from a starting node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start": { "type": "string", "description": "Starting node ID" },
                    "relation": {
                        "type": "string",
                        "description": "Relation type to follow",
                        "enum": ["caused_by", "led_to", "triggered", "part_of", "contains",
                                 "depends_on", "blocked_by", "supports", "contradicts",
                                 "refined_by", "used_for", "created_by", "modified_by", "resolved_by"]
                    },
                    "depth": { "type": "integer", "description": "Maximum traversal depth", "default": 2 }
                },
                "required": ["start", "relation"]
            }
        }),
        json!({
            "name": "acp_graph_remove_node",
            "description": "Remove a node and all its connected edges from the context graph.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Node ID to remove" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "acp_graph_remove_edge",
            "description": "Remove an edge from the context graph.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Edge ID to remove" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "acp_skill_register",
            "description": "Register a new reusable skill/procedure in agent memory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Skill name" },
                    "version": { "type": "string", "description": "Semantic version (e.g. 1.0.0)", "default": "1.0.0" },
                    "description": { "type": "string", "description": "What the skill does" },
                    "instruction": { "type": "string", "description": "Step-by-step instruction for executing the skill" },
                    "trigger": {
                        "type": "object",
                        "properties": {
                            "patterns": { "type": "array", "items": { "type": "object" }, "description": "Trigger patterns" },
                            "context_conditions": { "type": "array", "items": { "type": "object" } },
                            "explicit_invocation": { "type": "boolean", "default": false }
                        }
                    },
                    "dependencies": {
                        "type": "object",
                        "properties": {
                            "tools_required": { "type": "array", "items": { "type": "string" } },
                            "skills_required": { "type": "array", "items": { "type": "string" } },
                            "min_context_window": { "type": "integer" }
                        }
                    }
                },
                "required": ["name", "description", "instruction"]
            }
        }),
        json!({
            "name": "acp_skill_get",
            "description": "Get a skill by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Skill ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "acp_skill_list",
            "description": "List all registered skills.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "acp_skill_update",
            "description": "Update an existing skill.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Skill ID to update" },
                    "name": { "type": "string" },
                    "version": { "type": "string" },
                    "description": { "type": "string" },
                    "instruction": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "acp_skill_export",
            "description": "Export a skill as a portable package for sharing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Skill ID to export" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "acp_skill_resolve",
            "description": "Find matching skills for a given context query.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language query to match skills" },
                    "available_tools": { "type": "array", "items": { "type": "string" }, "description": "Tools available in current context" },
                    "session_tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for current session" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "acp_version_snapshot",
            "description": "Create a snapshot of the current agent memory state for later restore.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "reason": { "type": "string", "description": "Why this snapshot is being taken" }
                },
                "required": ["reason"]
            }
        }),
        json!({
            "name": "acp_version_restore",
            "description": "Restore agent memory to a previous snapshot. Entries not in the snapshot are soft-deleted.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "version": { "type": "string", "description": "Snapshot ID or version number to restore" }
                },
                "required": ["version"]
            }
        }),
        json!({
            "name": "acp_version_diff",
            "description": "Compare two snapshots to see what changed between them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": { "type": "string", "description": "Source snapshot ID or version" },
                    "to": { "type": "string", "description": "Target snapshot ID or version" }
                },
                "required": ["from", "to"]
            }
        }),
        json!({
            "name": "acp_version_list",
            "description": "List all available memory snapshots.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "acp_exchange_export",
            "description": "Export the complete agent state (episodes, knowledge, skills, graph) as a portable bundle.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "acp_exchange_import",
            "description": "Import an agent state bundle, merging episodes, knowledge, skills, and graph data.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "identity": { "type": "object", "description": "Agent identity" },
                    "episodes": { "type": "array", "description": "Episodes to import" },
                    "semantic_entries": { "type": "array", "description": "Semantic entries to import" },
                    "nodes": { "type": "array", "description": "Graph nodes to import" },
                    "edges": { "type": "array", "description": "Graph edges to import" },
                    "skills": { "type": "array", "description": "Skills to import" },
                    "snapshots": { "type": "array", "description": "Snapshots metadata" }
                },
                "required": ["identity"]
            }
        }),
        json!({
            "name": "acp_capabilities",
            "description": "List all ACP methods supported by this server with their conformance level.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "acp_health",
            "description": "Check server health and uptime.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "acp_skill_invoke",
            "description": "Invoke a registered skill by ID. Returns the instruction text and records the invocation in performance metrics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Skill ID" },
                    "input": { "type": "object", "description": "Input context for the skill (optional)" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "acp_memory_consolidate",
            "description": "Consolidate episodic memories into semantic knowledge. Promotes high-importance episodes to the semantic layer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Only consolidate episodes from this session (optional, omit for all sessions)"
                    },
                    "min_importance": {
                        "type": "number",
                        "description": "Minimum importance threshold (0.0-1.0, default 0.5)"
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "acp_memory_prune",
            "description": "Prune old or low-importance memories according to retention policy. Removes expired episodes, low-importance semantic entries, and orphan graph nodes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "episodic": {
                        "type": "object",
                        "properties": {
                            "max_episodes": { "type": "integer", "description": "Max episodes to keep" },
                            "max_age_days": { "type": "integer", "description": "Max age in days" }
                        }
                    },
                    "semantic": {
                        "type": "object",
                        "properties": {
                            "min_importance": { "type": "number", "description": "Min importance threshold (0.0-1.0)" }
                        }
                    },
                    "graph": {
                        "type": "object",
                        "properties": {
                            "prune_orphans": { "type": "boolean", "description": "Remove orphan nodes", "default": true }
                        }
                    }
                }
            }
        }),
        json!({
            "name": "acp_graph_merge",
            "description": "Merge an external context graph (e.g. from a peer agent) into the local graph, with a conflict strategy and optional namespace.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "external_graph": {
                        "type": "object",
                        "properties": {
                            "nodes": { "type": "array", "items": { "type": "object" } },
                            "edges": { "type": "array", "items": { "type": "object" } }
                        },
                        "description": "The nodes and edges to merge in"
                    },
                    "conflict_strategy": {
                        "type": "string",
                        "enum": ["prefer_local", "prefer_remote"],
                        "description": "Which side wins on id collision",
                        "default": "prefer_local"
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Optional prefix applied to incoming ids to avoid collisions"
                    }
                },
                "required": ["external_graph"]
            }
        }),
        json!({
            "name": "acp_version_branch",
            "description": "Create a named branch of cognitive state from a snapshot. Omit from_version to branch from the current state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Branch name" },
                    "from_version": { "type": "string", "description": "Snapshot id or version to branch from (optional)" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "acp_version_merge",
            "description": "Merge a branch back into the main line. prefer_branch adopts the branch state; prefer_main keeps the current state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "branch": { "type": "string", "description": "Branch name to merge" },
                    "strategy": {
                        "type": "string",
                        "enum": ["prefer_branch", "prefer_main"],
                        "description": "Conflict resolution strategy",
                        "default": "prefer_branch"
                    }
                },
                "required": ["branch"]
            }
        }),
        json!({
            "name": "acp_exchange_share",
            "description": "Selectively share memory layers with another agent, filtered by tags, with a granted access level. Produces a share manifest.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "recipient": { "type": "string", "description": "Recipient agent id/urn" },
                    "layers": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["episodic", "semantic", "graph", "procedural", "all"] },
                        "description": "Layers to share (default: all)"
                    },
                    "filter": {
                        "type": "object",
                        "properties": {
                            "tags": { "type": "array", "items": { "type": "string" } }
                        },
                        "description": "Optional tag filter applied to episodic/semantic entries"
                    },
                    "access_level": {
                        "type": "string",
                        "enum": ["read_only", "read_write"],
                        "description": "Access granted to the recipient",
                        "default": "read_only"
                    }
                },
                "required": ["recipient"]
            }
        }),
        json!({
            "name": "acp_exchange_sync",
            "description": "Bidirectionally sync agent memory with a peer. For pull/both, imports the peer's remote_bundle; for push/both, returns the local bundle so the peer can import it. Reports received and sent counts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "direction": {
                        "type": "string",
                        "enum": ["push", "pull", "both"],
                        "description": "push = send local bundle only; pull = import peer bundle only; both = do both",
                        "default": "both"
                    },
                    "remote_bundle": {
                        "type": "object",
                        "description": "Peer AgentBundle to import (required for pull/both to receive anything)"
                    }
                },
                "required": []
            }
        }),
    ]
}
