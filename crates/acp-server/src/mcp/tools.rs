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
    ]
}
