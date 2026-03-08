use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use rusqlite::Connection;

use acp_graph::GraphEngine;

use crate::schema;
use crate::AcpStoreError;

/// Configuration for the SQLite store.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub db_path: PathBuf,
    pub wal_mode: bool,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("acp.db"),
            wal_mode: true,
        }
    }
}

/// SQLite-backed storage implementing all ACP traits.
///
/// Includes an in-memory `GraphEngine` that is loaded from SQLite on startup
/// and kept in sync via write-through on mutations.
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    config: StoreConfig,
    graph: RwLock<GraphEngine>,
}

impl SqliteStore {
    /// Create a new store backed by a file.
    pub fn new(config: StoreConfig) -> Result<Self, AcpStoreError> {
        let conn = Connection::open(&config.db_path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            config,
            graph: RwLock::new(GraphEngine::new()),
        };
        store.initialize()?;
        store.load_graph()?;
        Ok(store)
    }

    /// Create an in-memory store (for tests).
    pub fn in_memory() -> Result<Self, AcpStoreError> {
        let conn = Connection::open_in_memory()?;
        let config = StoreConfig {
            db_path: PathBuf::from(":memory:"),
            wal_mode: false,
        };
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            config,
            graph: RwLock::new(GraphEngine::new()),
        };
        store.initialize()?;
        Ok(store)
    }

    /// Open an existing store or create a new one at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AcpStoreError> {
        Self::new(StoreConfig {
            db_path: path.as_ref().to_path_buf(),
            ..Default::default()
        })
    }

    fn initialize(&self) -> Result<(), AcpStoreError> {
        let conn = self.conn.lock().unwrap();

        if self.config.wal_mode {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;",
            )?;
        }

        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA cache_size = -64000;
             PRAGMA busy_timeout = 5000;",
        )?;

        schema::apply_schema(&conn)?;
        Ok(())
    }

    /// Get a reference to the connection (internal use).
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    /// Get a read lock on the graph engine.
    pub(crate) fn graph_read(&self) -> std::sync::RwLockReadGuard<'_, GraphEngine> {
        self.graph.read().unwrap()
    }

    /// Get a write lock on the graph engine.
    pub(crate) fn graph_write(&self) -> std::sync::RwLockWriteGuard<'_, GraphEngine> {
        self.graph.write().unwrap()
    }
}
