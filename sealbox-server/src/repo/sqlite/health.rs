use crate::repo::HealthRepo;

#[derive(Debug, Clone)]
pub(crate) struct SqliteHealthRepo;

use crate::error::Result;

impl HealthRepo for SqliteHealthRepo {
    fn check_health(&self, conn: &rusqlite::Connection) -> Result<bool> {
        let mut stmt = conn.prepare("SELECT 1")?;
        let row: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(row == 1)
    }
}
