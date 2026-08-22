use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;

use crate::{
    error::{Result, SealboxError},
    repo::{Job, JobRepo, JobStatus, Rotation},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteJobRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteJobRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                grant_name TEXT NOT NULL,
                params TEXT NOT NULL,
                runner TEXT NOT NULL,
                status TEXT NOT NULL,
                submitted_by TEXT NOT NULL,
                submitted_at INTEGER NOT NULL,
                claimed_at INTEGER,
                finished_at INTEGER,
                exit_code INTEGER,
                output TEXT,
                rotation TEXT
            )",
            (),
        )?;
        // The claim query filters on exactly this pair.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_jobs_runner_status ON jobs (runner, status)",
            (),
        )?;
        Ok(())
    }

    const COLUMNS: &'static str = "id, grant_name, params, runner, status, submitted_by, \
                                   submitted_at, claimed_at, finished_at, exit_code, output, \
                                   rotation";

    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
        let params: String = row.get(2)?;
        Ok(Job {
            id: row.get(0)?,
            grant: row.get(1)?,
            params: serde_json::from_str(&params).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            runner: row.get(3)?,
            status: row.get(4)?,
            submitted_by: row.get(5)?,
            submitted_at: row.get(6)?,
            claimed_at: row.get(7)?,
            finished_at: row.get(8)?,
            exit_code: row.get(9)?,
            output: row.get(10)?,
            rotation: row
                .get::<_, Option<String>>(11)?
                .map(|r| serde_json::from_str(&r))
                .transpose()
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        11,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
        })
    }
}

impl JobRepo for SqliteJobRepo {
    fn submit(
        &self,
        grant: &str,
        runner: &str,
        params: &BTreeMap<String, String>,
        by: &str,
        rotation: Option<&Rotation>,
    ) -> Result<Job> {
        let guard = self.conn.lock()?;
        let params_json = serde_json::to_string(params)
            .map_err(|e| SealboxError::DatabaseError(e.to_string()))?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        let rotation_json = rotation
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| SealboxError::DatabaseError(e.to_string()))?;

        guard.execute(
            "INSERT INTO jobs (grant_name, params, runner, status, submitted_by, submitted_at, rotation)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                grant,
                &params_json,
                runner,
                JobStatus::Pending,
                by,
                now,
                &rotation_json,
            ),
        )?;
        let id = guard.last_insert_rowid();

        let mut stmt =
            guard.prepare(&format!("SELECT {} FROM jobs WHERE id = ?1", Self::COLUMNS))?;
        Ok(stmt.query_one([id], Self::from_row)?)
    }

    fn get(&self, id: i64) -> Result<Option<Job>> {
        let guard = self.conn.lock()?;
        let mut stmt =
            guard.prepare(&format!("SELECT {} FROM jobs WHERE id = ?1", Self::COLUMNS))?;
        Ok(stmt.query_one([id], Self::from_row).optional()?)
    }

    /// One statement. The `WHERE id = (SELECT … LIMIT 1)` makes the write itself pick the winner,
    /// so two runners polling at the same moment cannot both come away with the same job. A
    /// read-then-write would need a transaction and a retry loop to say this less clearly.
    fn claim_next(&self, runner: &str) -> Result<Option<Job>> {
        let guard = self.conn.lock()?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        let claimed = guard.execute(
            "UPDATE jobs SET status = ?1, claimed_at = ?2
             WHERE id = (
                 SELECT id FROM jobs
                 WHERE runner = ?3 AND status = ?4
                 ORDER BY id LIMIT 1
             )",
            (JobStatus::Claimed, now, runner, JobStatus::Pending),
        )?;
        if claimed == 0 {
            return Ok(None);
        }

        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM jobs WHERE runner = ?1 AND status = ?2 ORDER BY claimed_at DESC, id DESC LIMIT 1",
            Self::COLUMNS
        ))?;
        Ok(stmt
            .query_one((runner, JobStatus::Claimed), Self::from_row)
            .optional()?)
    }

    fn report(&self, id: i64, runner: &str, exit_code: i32, output: &str) -> Result<Job> {
        let guard = self.conn.lock()?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let status = if exit_code == 0 {
            JobStatus::Succeeded
        } else {
            JobStatus::Failed
        };

        // The runner and status are part of the WHERE clause: a result is accepted only from the
        // runner that holds the claim, and only once.
        let updated = guard.execute(
            "UPDATE jobs SET status = ?1, finished_at = ?2, exit_code = ?3, output = ?4
             WHERE id = ?5 AND runner = ?6 AND status = ?7",
            (
                status,
                now,
                exit_code,
                output,
                id,
                runner,
                JobStatus::Claimed,
            ),
        )?;
        if updated == 0 {
            return Err(SealboxError::InvalidRequest(format!(
                "job {id} is not claimed by `{runner}`"
            )));
        }

        let mut stmt =
            guard.prepare(&format!("SELECT {} FROM jobs WHERE id = ?1", Self::COLUMNS))?;
        Ok(stmt.query_one([id], Self::from_row)?)
    }

    /// Marked failed, never re-queued: a grant is not necessarily idempotent, and silently
    /// re-running a `CREATE USER` or a deployment is worse than failing.
    fn fail_abandoned(&self, older_than: i64) -> Result<Vec<Job>> {
        let guard = self.conn.lock()?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM jobs WHERE status = ?1 AND claimed_at < ?2",
            Self::COLUMNS
        ))?;
        let abandoned: Vec<Job> = stmt
            .query_map((JobStatus::Claimed, older_than), Self::from_row)?
            .filter_map(|r| r.ok())
            .collect();

        for job in &abandoned {
            guard.execute(
                "UPDATE jobs SET status = ?1, finished_at = ?2, output = ?3 WHERE id = ?4",
                (
                    JobStatus::Failed,
                    now,
                    "the runner claimed this job and never reported; not retried",
                    job.id,
                ),
            )?;
        }
        Ok(abandoned)
    }
}
