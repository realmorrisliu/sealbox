use std::sync::{Arc, Mutex};

use crate::{
    error::Result,
    repo::{AuditFilter, AuditRecord, AuditRepo, NewAuditRecord},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteAuditRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteAuditRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                at INTEGER NOT NULL,
                identity TEXT,
                action TEXT NOT NULL,
                resource TEXT,
                outcome TEXT NOT NULL,
                detail TEXT
            )",
            (),
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_at ON audit (at DESC)",
            (),
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_identity ON audit (identity)",
            (),
        )?;
        Ok(())
    }
}

/// Append and read. There is deliberately no update or delete: the trail is the only account of
/// what an agent did, and an interface that could edit it would make it worth less than nothing.
impl AuditRepo for SqliteAuditRepo {
    fn append(&self, record: &NewAuditRecord) -> Result<()> {
        let guard = self.conn.lock()?;
        guard.execute(
            "INSERT INTO audit (at, identity, action, resource, outcome, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                time::OffsetDateTime::now_utc().unix_timestamp(),
                &record.identity,
                &record.action,
                &record.resource,
                &record.outcome,
                &record.detail,
            ),
        )?;
        Ok(())
    }

    fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditRecord>> {
        let guard = self.conn.lock()?;

        let mut sql = String::from(
            "SELECT id, at, identity, action, resource, outcome, detail FROM audit WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(identity) = &filter.identity {
            sql.push_str(" AND identity = ?");
            params.push(Box::new(identity.clone()));
        }
        if let Some(action) = &filter.action {
            sql.push_str(" AND action = ?");
            params.push(Box::new(action.clone()));
        }
        if let Some(since) = filter.since {
            sql.push_str(" AND at >= ?");
            params.push(Box::new(since));
        }
        sql.push_str(" ORDER BY at DESC, id DESC");
        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }

        let mut stmt = guard.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                at: row.get(1)?,
                identity: row.get(2)?,
                action: row.get(3)?,
                resource: row.get(4)?,
                outcome: row.get(5)?,
                detail: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
