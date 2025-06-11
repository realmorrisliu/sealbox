use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tracing::info;

use crate::{
    error::Result,
    repo::{Secret, SecretRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteSecretRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteSecretRepo {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode to improve concurrency
        conn.pragma_update(None, "journal_mode", &"WAL")?;

        // Set busy timeout to prevent immediate failure on lock conflicts
        conn.busy_timeout(std::time::Duration::from_millis(500))?;

        // Initialize database table structure
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secrets (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                metadata TEXT,
                access_count INTEGER DEFAULT 0,
                PRIMARY KEY (namespace, key, version)
            )",
            (),
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

impl SecretRepo for SqliteSecretRepo {
    fn get_secret(&self, key: &str) -> Option<Secret> {
        info!("get_secret");
        let _conn = self.conn.lock().unwrap();
        todo!()
    }

    fn save_secret(&self, secret: &Secret) {
        info!("save_secret");
        let _conn = self.conn.lock().unwrap();
        todo!()
    }

    fn delete_secret(&self, key: &str) {
        info!("delete_secret");
        let _conn = self.conn.lock().unwrap();
        todo!()
    }
}
