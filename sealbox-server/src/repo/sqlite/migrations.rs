use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;

use crate::{
    error::{Result, SealboxError},
    repo::{LEGACY_TENANT_ID, TenantStatus},
};

use super::{SqliteMasterKeyRepo, SqliteSecretRepo, SqliteTenantRepo};

const TENANT_SCOPE_MIGRATION: i64 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    pub schema_version: i64,
    pub migration_required: bool,
    pub legacy_data_present: bool,
    pub master_key_count: i64,
    pub secret_version_count: i64,
    pub empty_secret_namespace_count: i64,
    pub orphan_secret_count: i64,
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    if !table_exists(conn, table)? {
        return Ok(false);
    }
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(columns.iter().any(|candidate| candidate == column))
}

fn table_count(conn: &Connection, table: &str) -> Result<i64> {
    if !table_exists(conn, table)? {
        return Ok(0);
    }
    Ok(
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

pub fn inspect_migration(conn: &Connection) -> Result<MigrationReport> {
    let schema_version = if table_exists(conn, "schema_migrations")? {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?
    } else {
        0
    };
    let master_key_count = table_count(conn, "master_keys")?;
    let secret_version_count = table_count(conn, "secrets")?;
    let secret_has_namespace = column_exists(conn, "secrets", "namespace")?;
    let master_key_has_namespace = column_exists(conn, "master_keys", "namespace")?;
    let empty_secret_namespace_count = if secret_has_namespace {
        conn.query_row(
            "SELECT COUNT(*) FROM secrets WHERE namespace = ''",
            [],
            |row| row.get(0),
        )?
    } else {
        0
    };
    let orphan_secret_count = if secret_version_count == 0 || master_key_count == 0 {
        secret_version_count
    } else if master_key_has_namespace && secret_has_namespace {
        conn.query_row(
            "SELECT COUNT(*)
             FROM secrets AS s
             LEFT JOIN master_keys AS m
               ON m.id = s.master_key_id AND m.namespace = s.namespace
             WHERE m.id IS NULL",
            [],
            |row| row.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*)
             FROM secrets AS s
             LEFT JOIN master_keys AS m ON m.id = s.master_key_id
             WHERE m.id IS NULL",
            [],
            |row| row.get(0),
        )?
    };
    Ok(MigrationReport {
        schema_version,
        migration_required: schema_version < TENANT_SCOPE_MIGRATION,
        legacy_data_present: master_key_count > 0 || secret_version_count > 0,
        master_key_count,
        secret_version_count,
        empty_secret_namespace_count,
        orphan_secret_count,
    })
}

pub fn inspect_migration_path(path: &str) -> Result<MigrationReport> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    inspect_migration(&conn)
}

pub fn backup_before_migration(conn: &Connection, database_path: &str) -> Result<Option<PathBuf>> {
    let report = inspect_migration(conn)?;
    if !report.migration_required || !report.legacy_data_present || database_path == ":memory:" {
        return Ok(None);
    }
    if report.orphan_secret_count > 0 {
        return Err(SealboxError::DatabaseError(format!(
            "legacy migration found {} secret version(s) without a matching master key",
            report.orphan_secret_count
        )));
    }
    let source = Path::new(database_path);
    let backup = PathBuf::from(format!("{}.pre-tenant-v2.bak", source.display()));
    if backup.exists() {
        return Ok(Some(backup));
    }
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SealboxError::DatabaseError(error.to_string()))?;
    }
    conn.execute("VACUUM main INTO ?1", [backup.to_string_lossy().as_ref()])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&backup, fs::Permissions::from_mode(0o600))
            .map_err(|error| SealboxError::DatabaseError(error.to_string()))?;
    }
    Ok(Some(backup))
}

pub(crate) fn run_migrations(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON; BEGIN IMMEDIATE;")?;
    let migration_result = (|| -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
            (),
        )?;
        SqliteTenantRepo::init_tables(conn)?;
        SqliteMasterKeyRepo::init_table(conn)?;
        SqliteSecretRepo::init_table(conn)?;

        let orphan_secret_count = inspect_migration(conn)?.orphan_secret_count;
        if orphan_secret_count > 0 {
            return Err(SealboxError::DatabaseError(format!(
                "tenant migration found {orphan_secret_count} secret version(s) without a master key in the same namespace"
            )));
        }

        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        conn.execute(
            "INSERT OR IGNORE INTO tenants (id, display_name, status, created_at, updated_at)
             VALUES (?1, 'Legacy single-tenant data', ?2, ?3, ?3)",
            (LEGACY_TENANT_ID, TenantStatus::Active, now),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            (TENANT_SCOPE_MIGRATION, now),
        )?;
        Ok(())
    })();

    match migration_result {
        Ok(()) => {
            conn.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_existing_empty_namespace_and_global_master_key() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE master_keys (
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT
            );
            CREATE UNIQUE INDEX idx_master_keys_one_active
                ON master_keys(status) WHERE status = 'Active';
            CREATE TABLE secrets (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                master_key_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                metadata TEXT,
                PRIMARY KEY (namespace, key, version)
            );",
        )
        .unwrap();
        let key_id = uuid::Uuid::new_v4();
        conn.execute(
            "INSERT INTO master_keys (id, public_key, created_at, status)
             VALUES (?1, 'public', 1, 'Active')",
            [key_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_keys (id, public_key, created_at, status)
             VALUES (?1, 'retired-public', 0, 'Retired')",
            [uuid::Uuid::new_v4()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO secrets (
                namespace, key, version, encrypted_data, encrypted_data_key,
                master_key_id, created_at, updated_at
             ) VALUES ('', 'example', 1, X'01', X'02', ?1, 1, 1)",
            [key_id],
        )
        .unwrap();

        run_migrations(&conn).unwrap();

        let master_namespace: String = conn
            .query_row("SELECT namespace FROM master_keys", [], |row| row.get(0))
            .unwrap();
        let secret_namespace: String = conn
            .query_row("SELECT namespace FROM secrets", [], |row| row.get(0))
            .unwrap();
        assert_eq!(master_namespace, LEGACY_TENANT_ID);
        assert_eq!(secret_namespace, LEGACY_TENANT_ID);
    }

    #[test]
    fn reports_and_backs_up_populated_legacy_database_without_mutating_it() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("sealbox.db");
        let conn = rusqlite::Connection::open(&database).unwrap();
        conn.execute_batch(
            "CREATE TABLE master_keys (
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT
            );
            CREATE TABLE secrets (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                version INTEGER NOT NULL,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                master_key_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                metadata TEXT,
                PRIMARY KEY (namespace, key, version)
            );",
        )
        .unwrap();
        let key_id = uuid::Uuid::new_v4();
        conn.execute(
            "INSERT INTO master_keys (id, public_key, created_at, status)
             VALUES (?1, 'public', 1, 'Active')",
            [key_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_keys (id, public_key, created_at, status)
             VALUES (?1, 'retired-public', 0, 'Retired')",
            [uuid::Uuid::new_v4()],
        )
        .unwrap();
        for version in 1..=2 {
            conn.execute(
                "INSERT INTO secrets (
                    namespace, key, version, encrypted_data, encrypted_data_key,
                    master_key_id, created_at, updated_at, expires_at
                 ) VALUES ('', 'example', ?1, X'01', X'02', ?2, 1, 1, ?3)",
                rusqlite::params![version, key_id, if version == 1 { Some(2) } else { None }],
            )
            .unwrap();
        }

        let report = inspect_migration(&conn).unwrap();
        assert!(report.migration_required);
        assert_eq!(report.master_key_count, 2);
        assert_eq!(report.secret_version_count, 2);
        assert_eq!(report.empty_secret_namespace_count, 2);
        let backup = backup_before_migration(&conn, database.to_str().unwrap())
            .unwrap()
            .unwrap();

        assert!(backup.is_file());
        let backup_report = inspect_migration_path(backup.to_str().unwrap()).unwrap();
        assert_eq!(backup_report.secret_version_count, 2);
        assert_eq!(backup_report.empty_secret_namespace_count, 2);
        assert!(!column_exists(&conn, "master_keys", "namespace").unwrap());

        run_migrations(&conn).unwrap();
        let migrated = inspect_migration(&conn).unwrap();
        assert!(!migrated.migration_required);
        assert_eq!(migrated.master_key_count, 2);
        assert_eq!(migrated.secret_version_count, 2);
        assert_eq!(migrated.empty_secret_namespace_count, 0);
        assert_eq!(migrated.orphan_secret_count, 0);
    }

    #[test]
    fn refuses_to_migrate_orphaned_legacy_secrets() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE master_keys (
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL
            );
            CREATE TABLE secrets (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                version INTEGER NOT NULL,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                master_key_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (namespace, key, version)
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO secrets (
                namespace, key, version, encrypted_data, encrypted_data_key,
                master_key_id, created_at, updated_at
             ) VALUES ('', 'orphan', 1, X'01', X'02', ?1, 1, 1)",
            [uuid::Uuid::new_v4()],
        )
        .unwrap();

        let error = run_migrations(&conn).unwrap_err().to_string();

        assert!(error.contains("without a master key"));
        assert!(!table_exists(&conn, "schema_migrations").unwrap());
    }
}
