use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use cortex_core::error::{CortexError, Result};

use crate::migrations;

pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| CortexError::Database(e.to_string()))?;

        // Enable WAL mode for concurrent reads during writes
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        migrations::run(&store)?;

        Ok(store)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| CortexError::Database(e.to_string()))?;

        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        migrations::run(&store)?;

        Ok(store)
    }

    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database lock poisoned")
    }
}
