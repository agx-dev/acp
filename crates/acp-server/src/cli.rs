use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "acp-server",
    about = "Agent Context Protocol — memory server for AI agents",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Storage directory for agent memory
    #[arg(long, global = true, default_value = ".acp", env = "ACP_STORAGE")]
    pub storage: PathBuf,

    /// Embedding provider (mock for testing)
    #[arg(long, global = true, default_value = "mock", env = "ACP_EMBEDDING_PROVIDER")]
    pub embedding_provider: String,

    /// Transport (stdio)
    #[arg(long, global = true, default_value = "stdio")]
    pub transport: String,

    /// Log level
    #[arg(long, global = true, default_value = "info", env = "ACP_LOG_LEVEL")]
    pub log_level: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the server (default)
    Serve,
    /// Show memory statistics
    Stats,
}
