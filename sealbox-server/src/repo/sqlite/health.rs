use crate::repo::HealthRepo;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(crate) struct SqliteHealthRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteHealthRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }
}

use crate::error::Result;

impl HealthRepo for SqliteHealthRepo {
    fn check_health(&self) -> Result<bool> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        let mut stmt = conn.prepare("SELECT 1")?;
        let row: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(row == 1)
    }
}
