use std::sync::{Arc, Mutex};
use tracing::info;

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use uuid::Uuid;

use crate::{
    config::SealboxConfig,
    crypto::master_key::PrivateMasterKey,
    error::{Result, SealboxError},
    repo::{
        AuditRepo, AuthenticatorRepo, GrantRepo, HealthRepo, IdentityRepo, JobRepo, MasterKeyRepo,
        MasterKeyStatus, SecretRepo, SqliteAuditRepo, SqliteAuthenticatorRepo, SqliteGrantRepo,
        SqliteHealthRepo, SqliteIdentityRepo, SqliteJobRepo, SqliteMasterKeyRepo, SqliteSecretRepo,
        create_db_connection,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<SealboxConfig>,
    pub(crate) health_repo: Arc<dyn HealthRepo>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
    pub(crate) identity_repo: Arc<dyn IdentityRepo>,
    pub(crate) audit_repo: Arc<dyn AuditRepo>,
    pub(crate) grant_repo: Arc<dyn GrantRepo>,
    pub(crate) job_repo: Arc<dyn JobRepo>,
    pub(crate) authenticator_repo: Arc<dyn AuthenticatorRepo>,
    /// Challenges, sessions, enrolments, and pending approvals — all in memory, deliberately.
    pub(crate) passkey: crate::api::passkey::PasskeyState,
    /// When the bootstrap token stops being accepted. Stored as a deadline rather than a start
    /// time so it is directly assertable, and so a token left in the environment after use —
    /// the normal outcome — stops being useful on its own.
    pub(crate) bootstrap_deadline: Instant,
    /// Private halves of the server's own master keys, by id. Only rekey needs these; a key
    /// absent from this map is cold, and nothing can decrypt secrets under it.
    pub(crate) server_keys: Arc<HashMap<Uuid, PrivateMasterKey>>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let conn = create_db_connection(&config.store_path)?;

        SqliteSecretRepo::init_table(&conn)?;
        SqliteMasterKeyRepo::init_table(&conn)?;
        SqliteIdentityRepo::init_table(&conn)?;
        SqliteAuditRepo::init_table(&conn)?;
        SqliteGrantRepo::init_table(&conn)?;
        SqliteJobRepo::init_table(&conn)?;
        SqliteAuthenticatorRepo::init_table(&conn)?;

        // One connection, shared by the repositories. Nothing above this layer holds it:
        // a database lock has no business in an HTTP handler.
        let conn = Arc::new(Mutex::new(conn));

        let state = Self {
            config: Arc::new(config.clone()),
            health_repo: Arc::new(SqliteHealthRepo::new(conn.clone())),
            secret_repo: Arc::new(SqliteSecretRepo::new(conn.clone())),
            master_key_repo: Arc::new(SqliteMasterKeyRepo::new(conn.clone())),
            identity_repo: Arc::new(SqliteIdentityRepo::new(conn.clone())),
            audit_repo: Arc::new(SqliteAuditRepo::new(conn.clone())),
            grant_repo: Arc::new(SqliteGrantRepo::new(conn.clone())),
            job_repo: Arc::new(SqliteJobRepo::new(conn.clone())),
            authenticator_repo: Arc::new(SqliteAuthenticatorRepo::new(conn)),
            passkey: crate::api::passkey::PasskeyState::new(&config.public_url)?,
            server_keys: Arc::new(HashMap::new()),
            bootstrap_deadline: Instant::now() + config.bootstrap_window,
        };

        let server_keys = state.load_server_master_keys()?;
        let state = Self {
            server_keys: Arc::new(server_keys),
            ..state
        };

        // Perform startup cleanup of expired secrets
        state.startup_cleanup()?;

        Ok(state)
    }

    /// Load the server's master keys and make sure each is registered.
    ///
    /// The first is Active — new secrets are encrypted under it. Any others are Retired: still
    /// loaded, so the secrets already under them stay readable and can be rekeyed onto the
    /// current key. Remove a path from the list once nothing references it.
    ///
    /// A missing or unreadable file is fatal rather than a cue to generate one: silently
    /// creating a key when the path is wrong would leave every existing secret cold, encrypted
    /// under a key nobody has, and the failure would only surface later on a read.
    ///
    /// The exception is a server that has never held anything — see `generate_first_master_key`.
    fn load_server_master_keys(&self) -> Result<HashMap<Uuid, PrivateMasterKey>> {
        let mut loaded = HashMap::new();
        self.generate_first_master_key()?;

        for (index, path) in self.config.master_key_paths.iter().enumerate() {
            let pem = std::fs::read_to_string(path).map_err(|e| {
                SealboxError::ConfigError(format!(
                    "Cannot read the server master key at {path}: {e}. Generate one and point \
                     SEALBOX_MASTER_KEY_PATH at it; sealbox will not create it for you, because \
                     doing so on a mistyped path would silently make every stored secret cold."
                ))
            })?;

            let private_key = PrivateMasterKey::from_str(&pem)?;
            let public_pem = private_key.public_key_pem()?;
            let status = if index == 0 {
                MasterKeyStatus::Active
            } else {
                MasterKeyStatus::Retired
            };
            let key = self
                .master_key_repo
                .ensure_server_held(&public_pem, status)?;
            info!(
                "Server master key {} loaded from {} ({})",
                key.id,
                path,
                if index == 0 { "current" } else { "retired" }
            );
            loaded.insert(key.id, private_key);
        }

        Ok(loaded)
    }

    /// Write a master key at the configured path, but only on a server that has never held
    /// anything.
    ///
    /// The reasoning that makes a missing key fatal is about *existing* secrets: a key generated
    /// at a mistyped path would leave them cold. That condition is absent on a first boot, and its
    /// absence is what this checks — no master key and no secret in the store, one path
    /// configured, no file at it. Anything else falls through to the fatal path above.
    ///
    /// The case worth being careful about is a volume lost and a database restored from
    /// replication: secrets exist, the key file does not, and generating one there would replace
    /// the only thing that can read them. That is why the question asked is "has this server ever
    /// held anything", not "is the file missing".
    ///
    /// Hosted platforms are why this exists at all: a volume exists only while a machine is
    /// attached, so there is no moment before first boot in which to place a file.
    fn generate_first_master_key(&self) -> Result<()> {
        let [path] = self.config.master_key_paths.as_slice() else {
            // A rotation list on a fresh server is a mistake rather than an intent, and it is
            // cheap to correct. Fall through and let the missing file be fatal.
            return Ok(());
        };
        if std::path::Path::new(path).exists() {
            return Ok(());
        }
        if !self.master_key_repo.fetch_all_master_keys()?.is_empty()
            || self.secret_repo.count_secrets()? > 0
        {
            return Ok(());
        }

        let (private_pem, public_pem) = crate::crypto::master_key::generate_key_pair()?;
        write_private_key(path, &private_pem)?;

        // The fingerprint, never the key. Logs get shipped, aggregated, retained, and read by
        // people who should not hold the key to every credential in the system (ADR 0010).
        info!(
            "No master key at {path} and nothing stored yet — generated one. Fingerprint {}. \
             Back it up now: it is the only copy, and replication covers the database, not this.",
            fingerprint(&public_pem)
        );
        Ok(())
    }

    /// Mark jobs a runner claimed and never reported as failed.
    ///
    /// Never re-queued: a grant is not necessarily idempotent, and silently re-running a
    /// `CREATE USER` or a deployment is worse than failing. A caller who wants it tried again
    /// can submit another job, having decided that is safe.
    pub fn sweep_abandoned_jobs(&self, timeout: std::time::Duration) -> Result<usize> {
        let cutoff = time::OffsetDateTime::now_utc().unix_timestamp() - timeout.as_secs() as i64;
        let abandoned = self.job_repo.fail_abandoned(cutoff)?;

        for job in &abandoned {
            info!(
                "Job {} was claimed by runner '{}' and never reported; marked failed",
                job.id, job.runner
            );
            self.audit_repo.append(&crate::repo::NewAuditRecord {
                identity: Some(job.runner.clone()),
                action: "job.abandoned".to_string(),
                resource: Some(job.grant.clone()),
                outcome: crate::repo::AuditOutcome::Failed,
                detail: Some(format!("job {} was never reported; not retried", job.id)),
            })?;
        }
        Ok(abandoned.len())
    }

    /// Clean up expired secrets during application startup
    fn startup_cleanup(&self) -> Result<()> {
        info!("Performing startup cleanup of expired secrets...");
        let deleted_count = self.secret_repo.cleanup_expired_secrets()?;
        if deleted_count > 0 {
            info!(
                "Startup cleanup completed: removed {} expired secrets",
                deleted_count
            );
        } else {
            info!("Startup cleanup completed: no expired secrets found");
        }
        Ok(())
    }
}

/// Write with the file created `0600` from the start, rather than created and then tightened —
/// a window in which a key is world-readable is a window.
fn write_private_key(path: &str, pem: &str) -> Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| {
        SealboxError::ConfigError(format!("Cannot create a master key at {path}: {e}"))
    })?;
    file.write_all(pem.as_bytes())
        .map_err(|e| SealboxError::ConfigError(format!("Cannot write the master key: {e}")))?;
    Ok(())
}

/// A short, stable name for a key that discloses nothing about it. Enough to tell two keys apart
/// in a log, and to check that the file on the volume is the one being used.
fn fingerprint(public_pem: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(public_pem.as_bytes());
    digest
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SealboxConfig;

    /// A config pointing at a fresh temp directory, with `n` master key paths configured.
    fn config(dir: &tempfile::TempDir, paths: &[&str]) -> SealboxConfig {
        SealboxConfig {
            public_url: "http://localhost:8080".to_string(),
            bootstrap_token: None,
            store_path: dir.path().join("test.db").to_string_lossy().into_owned(),
            listen_addr: "127.0.0.1:0".to_string(),
            master_key_paths: paths
                .iter()
                .map(|p| dir.path().join(p).to_string_lossy().into_owned())
                .collect(),
            bootstrap_window: std::time::Duration::from_secs(1800),
        }
    }

    #[test]
    fn a_fresh_server_generates_its_own_key() {
        let dir = tempfile::tempdir().expect("Should create a temp dir");
        let config = config(&dir, &["master.pem"]);

        AppState::new(&config).expect("a fresh server should bring itself up");

        let written = dir.path().join("master.pem");
        assert!(written.exists(), "the key should have been written");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&written).unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o600,
                "a key readable by anyone else is a leak"
            );
        }
    }

    #[test]
    fn a_second_start_reuses_the_key_it_generated() {
        let dir = tempfile::tempdir().expect("Should create a temp dir");
        let config = config(&dir, &["master.pem"]);

        AppState::new(&config).expect("first start");
        let first = std::fs::read_to_string(dir.path().join("master.pem")).unwrap();

        AppState::new(&config).expect("second start");
        let second = std::fs::read_to_string(dir.path().join("master.pem")).unwrap();

        assert_eq!(first, second, "a restart must not replace the key");
    }

    #[test]
    fn a_restored_database_without_its_key_refuses_to_start() {
        let dir = tempfile::tempdir().expect("Should create a temp dir");
        let config = config(&dir, &["master.pem"]);

        // Stand in for a volume lost and a database restored from replication: secrets exist,
        // the key file does not. Generating one here would replace the only thing that can read
        // them, and the failure would surface later, on a read, to someone who did not cause it.
        let state = AppState::new(&config).expect("first start");
        let master_key = state.master_key_repo.get_valid_master_key().unwrap();
        state
            .secret_repo
            .create_new_version(
                "app/db-url",
                &crate::repo::SecretValue::Supplied("hunter2".to_string()),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should store a secret");
        drop(state);
        std::fs::remove_file(dir.path().join("master.pem")).unwrap();

        let err = match AppState::new(&config) {
            Err(e) => e,
            Ok(_) => panic!("this must not silently generate"),
        };
        assert!(
            err.to_string().contains("master.pem"),
            "the error should name the path: {err}"
        );
    }

    #[test]
    fn a_rotation_list_on_a_fresh_server_refuses() {
        let dir = tempfile::tempdir().expect("Should create a temp dir");
        let config = config(&dir, &["new.pem", "old.pem"]);

        // Two paths on a server that has never held anything is ambiguous — and cheap to correct
        // now, expensive to discover later.
        assert!(AppState::new(&config).is_err());
        assert!(!dir.path().join("new.pem").exists());
    }
}
