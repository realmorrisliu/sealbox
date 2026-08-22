use std::collections::BTreeMap;
use std::str::FromStr;

use rusqlite::{ToSql, types::FromSql};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    crypto::{
        data_key::DataKey,
        master_key::{PrivateMasterKey, PublicMasterKey},
    },
    error::{Result, SealboxError},
};

pub(crate) use self::sqlite::{
    SqliteAuditRepo, SqliteGrantRepo, SqliteHealthRepo, SqliteIdentityRepo, SqliteJobRepo,
    SqliteMasterKeyRepo, SqliteSecretRepo, create_db_connection,
};

pub mod adapter;
mod sqlite;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    pub key: String,             // Secret key identifier
    pub version: i32,            // Latest version number
    pub created_at: i64,         // Creation timestamp (Unix time)
    pub updated_at: i64,         // Last update timestamp (Unix time)
    pub expires_at: Option<i64>, // Expiry timestamp (Unix time), optional for TTL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub key: String,                 // Secret key identifier
    pub version: i32,                // Version number, incremented on each insert
    pub encrypted_data: Vec<u8>,     // The encrypted secret value
    pub encrypted_data_key: Vec<u8>, // The data key encrypted with user's public key
    pub master_key_id: Uuid,         // References master_keys.id (public key used)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub updated_at: i64,             // Last update timestamp (Unix time)
    pub expires_at: Option<i64>,     // Expiry timestamp (Unix time), optional for TTL
    pub metadata: Option<String>,    // Optional metadata in serialized format
}

impl Secret {
    /// Creates a new `Secret` instance by encrypting the provided data with a randomly generated data key,
    /// and then encrypting that data key with the provided master key's public key.
    ///
    /// # Arguments
    ///
    /// * `key` - The identifier for the secret.
    /// * `data` - The plaintext data to be encrypted and stored.
    /// * `master_key` - The `MasterKey` used to encrypt the data key.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Self>` containing the new `Secret` on success, or a `SealboxError` on failure.
    ///
    /// # Logic
    ///
    /// 1. Converts the input data to bytes.
    /// 2. Generates a random data key for encrypting the secret data.
    /// 3. Encrypts the secret data using the generated data key.
    /// 4. Encrypts the data key using the provided master key's public key.
    /// 5. Sets the creation and update timestamps to the current time.
    /// 6. Constructs and returns the new `Secret` instance.
    pub(crate) fn new(
        key: &str,
        data: &str,
        master_key: MasterKey,
        version: i32,
        ttl: Option<i64>,
    ) -> Result<Self> {
        let data_bytes = data.as_bytes();

        let data_key = DataKey::new();
        let encrypted_data = data_key.encrypt(data_bytes)?;

        let pub_key = PublicMasterKey::from_str(&master_key.public_key)?;
        let encrypted_data_key = pub_key.encrypt(data_key.as_bytes())?;

        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

        let expires_at = ttl.map(|ttl| now_timestamp + ttl);

        Ok(Self {
            key: key.to_string(),
            version,
            encrypted_data,
            encrypted_data_key,
            master_key_id: master_key.id,
            created_at: now_timestamp,
            updated_at: now_timestamp,
            expires_at,
            metadata: None,
        })
    }

    pub(crate) fn rekey(
        self,
        old_master_key_id: &Uuid,
        old_private_key: &PrivateMasterKey,
        new_master_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Self> {
        let mut secret = self.clone();

        if secret.master_key_id == *new_master_key_id {
            return Ok(secret);
        }

        if secret.master_key_id != *old_master_key_id {
            return Err(SealboxError::MasterKeyMismatch(
                secret.key,
                old_master_key_id.to_string(),
                secret.master_key_id.to_string(),
            ));
        }

        let new_pub_key = PublicMasterKey::from_str(new_public_key_pem)?;

        let data_key = old_private_key.decrypt(&secret.encrypted_data_key)?;
        let new_encrypted_data_key = new_pub_key.encrypt(&data_key)?;

        secret.encrypted_data_key = new_encrypted_data_key;
        secret.master_key_id = *new_master_key_id;
        secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();

        Ok(secret)
    }
}

/// What an identity is allowed to do. Ordered: each role admits everything the one below it can
/// do. Three roles with a natural inclusion order need no permission matrix, and a matrix would
/// invite per-resource entries — the boundary this design relies on is the grant, not an ACL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    /// Claims jobs addressed to it and reports results. **Disjoint from the others**, not
    /// beneath them: it cannot invoke a grant, read a secret by name, list secrets, or read the
    /// audit trail — and no other role can claim a job. Ordered lowest so that every threshold
    /// gate refuses it without needing to know about it.
    Runner,
    /// Invoke approved capabilities and read metadata. Nothing else.
    Agent,
    /// Additionally store secrets.
    Operator,
    /// Additionally manage identities and approve capabilities.
    Admin,
}

impl ToSql for Role {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Role::Runner => Ok(rusqlite::types::ToSqlOutput::from("Runner")),
            Role::Agent => Ok(rusqlite::types::ToSqlOutput::from("Agent")),
            Role::Operator => Ok(rusqlite::types::ToSqlOutput::from("Operator")),
            Role::Admin => Ok(rusqlite::types::ToSqlOutput::from("Admin")),
        }
    }
}

impl FromSql for Role {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str() {
            Ok("Runner") => Ok(Role::Runner),
            Ok("Agent") => Ok(Role::Agent),
            Ok("Operator") => Ok(Role::Operator),
            Ok("Admin") => Ok(Role::Admin),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl FromStr for Role {
    type Err = SealboxError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "runner" => Ok(Role::Runner),
            "agent" => Ok(Role::Agent),
            "operator" => Ok(Role::Operator),
            "admin" => Ok(Role::Admin),
            other => Err(SealboxError::InvalidRole(other.to_string())),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Role::Runner => "Runner",
            Role::Agent => "Agent",
            Role::Operator => "Operator",
            Role::Admin => "Admin",
        })
    }
}

/// A named caller. Its credential is stored only as a hash; the plaintext exists exactly once,
/// in the response that created it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
    #[serde(skip)]
    pub token_hash: Vec<u8>,
    pub created_at: i64,
    /// Set rather than deleting the row, so audit records naming this identity stay meaningful.
    pub revoked_at: Option<i64>,
}

impl Identity {
    /// Build an identity and return it alongside the one and only copy of its plaintext token.
    ///
    /// The token is 256 bits from a CSPRNG. It carries a `sealbox_` prefix so that a leaked one
    /// is recognisable to a secret scanner and to a human reading a config file.
    pub(crate) fn new(name: String, role: Role) -> Result<(Self, String)> {
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        let token = format!("sealbox_{}", hex_encode(&bytes));

        let identity = Self {
            id: Uuid::new_v4(),
            name,
            role,
            token_hash: hash_token(&token),
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            revoked_at: None,
        };
        Ok((identity, token))
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

/// SHA-256 of a token. Not a password KDF: the input is 256 random bits, so guessing is already
/// impossible and a deliberately slow hash would only add latency to every request. Lookup is by
/// hash, which is a single indexed query rather than a scan comparing candidates.
pub(crate) fn hash_token(token: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(token.as_bytes()).to_vec()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) trait IdentityRepo: Send + Sync {
    /// Store a new identity. The caller holds the only copy of the plaintext token.
    fn create(&self, identity: &Identity) -> Result<()>;
    /// Resolve a presented token to a live identity. Returns `None` for unknown or revoked.
    fn find_by_token(&self, token: &str) -> Result<Option<Identity>>;
    fn list(&self) -> Result<Vec<Identity>>;
    fn revoke(&self, name: &str) -> Result<()>;
    /// Whether any identity exists at all. The bootstrap path turns on this.
    fn any_exists(&self) -> Result<bool>;
}

/// One recorded attempt. The identity is stored by name rather than by reference so the record
/// stays readable after that identity is revoked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: i64,
    pub at: i64,
    /// `None` for an attempt that never authenticated.
    pub identity: Option<String>,
    pub action: String,
    pub resource: Option<String>,
    pub outcome: AuditOutcome,
    /// A short message. Never a secret value, a credential, or key material.
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Allowed,
    Unauthenticated,
    Forbidden,
    Failed,
}

impl ToSql for AuditOutcome {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            AuditOutcome::Allowed => Ok(rusqlite::types::ToSqlOutput::from("Allowed")),
            AuditOutcome::Unauthenticated => {
                Ok(rusqlite::types::ToSqlOutput::from("Unauthenticated"))
            }
            AuditOutcome::Forbidden => Ok(rusqlite::types::ToSqlOutput::from("Forbidden")),
            AuditOutcome::Failed => Ok(rusqlite::types::ToSqlOutput::from("Failed")),
        }
    }
}

impl FromSql for AuditOutcome {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str() {
            Ok("Allowed") => Ok(AuditOutcome::Allowed),
            Ok("Unauthenticated") => Ok(AuditOutcome::Unauthenticated),
            Ok("Forbidden") => Ok(AuditOutcome::Forbidden),
            Ok("Failed") => Ok(AuditOutcome::Failed),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

/// What to select when reading the trail. All fields optional; an empty filter reads everything,
/// most recent first.
#[derive(Debug, Default, Clone)]
pub struct AuditFilter {
    pub identity: Option<String>,
    pub action: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<usize>,
}

/// Append-only by construction: there is no method here that updates or removes a record.
pub(crate) trait AuditRepo: Send + Sync {
    fn append(&self, record: &NewAuditRecord) -> Result<()>;
    fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditRecord>>;
}

/// A record before it has an id or a timestamp.
#[derive(Debug, Clone)]
pub struct NewAuditRecord {
    pub identity: Option<String>,
    pub action: String,
    pub resource: Option<String>,
    pub outcome: AuditOutcome,
    pub detail: Option<String>,
}

/// The shortest generated value the system will produce. A caller asking for less is more
/// likely to have made a mistake than to have a reason, and a weak credential looks exactly
/// like a strong one from the outside.
pub const MIN_GENERATED_LENGTH: usize = 16;
/// Used when a caller does not say. 32 of the password alphabet is about 187 bits.
pub const DEFAULT_GENERATED_LENGTH: usize = 32;

/// Alphanumeric, minus the characters that get confused when a value is read aloud, retyped
/// from a screenshot, or pasted somewhere that mangles it: `0`/`O` and `1`/`l`/`I`.
///
/// No punctuation. Symbols in a generated credential cause trouble out of proportion to the
/// entropy they add — quoting in shells, escaping in connection strings, and YAML deciding a
/// value is something other than a string. Length is the cheaper way to buy entropy.
const PASSWORD_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GenerateKind {
    /// Printable, for a human or a connection string.
    Password,
    /// Raw randomness in hex, for machine consumption.
    Hex,
}

/// A request to have the server produce the value rather than be given one.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateSpec {
    #[serde(rename = "type")]
    pub kind: GenerateKind,
    pub length: Option<usize>,
}

impl GenerateSpec {
    /// Produce the value. Called where the encryption happens, so the plaintext exists only
    /// between here and the envelope — never assigned to a field, returned, or logged.
    pub(crate) fn generate(&self) -> Result<String> {
        let length = self.length.unwrap_or(DEFAULT_GENERATED_LENGTH);
        if length < MIN_GENERATED_LENGTH {
            return Err(SealboxError::InvalidRequest(format!(
                "generated length {length} is below the minimum of {MIN_GENERATED_LENGTH}"
            )));
        }

        let mut rng = rand::thread_rng();
        Ok(match self.kind {
            GenerateKind::Password => {
                use rand::Rng;
                (0..length)
                    .map(|_| PASSWORD_ALPHABET[rng.gen_range(0..PASSWORD_ALPHABET.len())] as char)
                    .collect()
            }
            GenerateKind::Hex => {
                let mut bytes = vec![0u8; length];
                rand::RngCore::fill_bytes(&mut rng, &mut bytes);
                bytes.iter().map(|b| format!("{b:02x}")).collect()
            }
        })
    }
}

/// Where a new version's value comes from.
#[derive(Debug, Clone)]
pub enum SecretValue {
    /// Handed in by a caller.
    Supplied(String),
    /// Produced by the server; the caller never sees it.
    Generated(GenerateSpec),
}

impl SecretValue {
    pub(crate) fn resolve(&self) -> Result<std::borrow::Cow<'_, str>> {
        match self {
            SecretValue::Supplied(value) => Ok(std::borrow::Cow::Borrowed(value)),
            SecretValue::Generated(spec) => Ok(std::borrow::Cow::Owned(spec.generate()?)),
        }
    }
}

/// Adapters are compiled in, not looked up (ADR 0007), so the known set is a constant. A grant
/// naming something else is refused at creation rather than at execution — where nobody would
/// be present to fix it.
pub const KNOWN_ADAPTERS: &[&str] = &["kubernetes-secret", "postgres-role"];

/// How a grant does its work. Modelled as an enum so that "both an adapter and a script" and
/// "neither" are unrepresentable rather than merely invalid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Implementation {
    /// A built-in, structurally limited to what its class of target system needs.
    Adapter {
        adapter: String,
        #[serde(default)]
        config: serde_json::Value,
    },
    /// The escape hatch. The body is stored here, never referenced by path: a grant pointing at
    /// a file could be approved once and the file edited afterwards, so what was reviewed and
    /// what runs would differ.
    Script {
        script: String,
        /// argv. Agent-supplied parameters are substituted into elements, never through a shell.
        command: Vec<String>,
    },
}

/// A permitted use of secrets: which ones, what is done with them, and where it runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub name: String,
    pub implementation: Implementation,
    /// Which runner executes this. Not resolved here — the runner does not exist yet.
    pub runner: String,
    /// Injected name to secret name. **This is what a human reviews**, and together with
    /// `files` the only secrets the implementation can reach.
    ///
    /// Each becomes an environment variable, and all of them together are also rendered into a
    /// `KEY=value` file whose path is available as `SEALBOX_ENVFILE` — one declaration, both
    /// forms, because a consumer needing `--from-env-file` should not require a second list.
    pub secrets: BTreeMap<String, String>,
    /// Secrets that must be a *file*: a kubeconfig, a docker config, an SSH key, a GCP
    /// service-account JSON. Each is written to a `0600` file whose path is substituted into
    /// argv as `{name}` and exported as `name`.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// Grants to run after this one succeeds, in order. Linear, stop-on-failure (ADR 0011).
    #[serde(default)]
    pub then: Vec<String>,
    pub created_at: i64,
    /// The identity that approved it. Kept for the audit question "who allowed this".
    pub created_by: String,
}

impl Grant {
    pub fn declares(&self, secret: &str) -> bool {
        self.secrets.values().any(|s| s == secret) || self.files.values().any(|s| s == secret)
    }

    /// Every secret this grant may reach, in either form.
    pub fn all_declared(&self) -> impl Iterator<Item = (&String, &String)> {
        self.secrets.iter().chain(self.files.iter())
    }
}

pub(crate) trait GrantRepo: Send + Sync {
    fn create(&self, grant: &Grant) -> Result<()>;
    fn get(&self, name: &str) -> Result<Option<Grant>>;
    fn list(&self) -> Result<Vec<Grant>>;
    fn remove(&self, name: &str) -> Result<()>;
}

/// One requested execution of a grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub grant: String,
    pub params: BTreeMap<String, String>,
    /// Which runner it is addressed to. Copied from the grant at submission, so a later change
    /// to the grant cannot redirect work already queued.
    pub runner: String,
    pub status: JobStatus,
    pub submitted_by: String,
    pub submitted_at: i64,
    pub claimed_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub exit_code: Option<i32>,
    pub output: Option<String>,
    /// Present when this job is a rotation.
    #[serde(default)]
    pub rotation: Option<Rotation>,
}

/// What a job is rotating, if it is. Carried alongside the job so that success or failure
/// decides the fate of a pending version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rotation {
    pub secret: String,
    /// The pending version created for this rotation.
    pub version: i32,
    /// Whether the grant's output becomes the value, rather than what the server generated.
    pub capture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Claimed,
    Succeeded,
    Failed,
}

impl JobStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, JobStatus::Succeeded | JobStatus::Failed)
    }
}

impl ToSql for JobStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            JobStatus::Pending => Ok(rusqlite::types::ToSqlOutput::from("Pending")),
            JobStatus::Claimed => Ok(rusqlite::types::ToSqlOutput::from("Claimed")),
            JobStatus::Succeeded => Ok(rusqlite::types::ToSqlOutput::from("Succeeded")),
            JobStatus::Failed => Ok(rusqlite::types::ToSqlOutput::from("Failed")),
        }
    }
}

impl FromSql for JobStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str() {
            Ok("Pending") => Ok(JobStatus::Pending),
            Ok("Claimed") => Ok(JobStatus::Claimed),
            Ok("Succeeded") => Ok(JobStatus::Succeeded),
            Ok("Failed") => Ok(JobStatus::Failed),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

/// What a runner is handed when it claims a job: the implementation, and the plaintext of only
/// the secrets that grant declares. There is no operation, for any role, that fetches a secret
/// by name — this is the only way plaintext leaves the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimedJob {
    pub id: i64,
    pub grant: String,
    pub params: BTreeMap<String, String>,
    pub implementation: Implementation,
    /// Injected name to plaintext value, for environment injection and the env-file. A rotation's
    /// new value arrives here as `SEALBOX_NEW`, indistinguishable from any declared secret — so
    /// there is nothing special for a script author to get right.
    pub secrets: BTreeMap<String, String>,
    /// Injected name to plaintext value, for secrets that must be a file.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// When true, the implementation's stdout becomes the secret's new value and must contain
    /// nothing else. Diagnostics belong on stderr.
    #[serde(default)]
    pub capture: bool,
}

pub(crate) trait JobRepo: Send + Sync {
    fn submit(
        &self,
        grant: &str,
        runner: &str,
        params: &BTreeMap<String, String>,
        by: &str,
        rotation: Option<&Rotation>,
    ) -> Result<Job>;
    fn get(&self, id: i64) -> Result<Option<Job>>;
    /// Claim the oldest pending job for this runner, atomically. `None` if there is none.
    fn claim_next(&self, runner: &str) -> Result<Option<Job>>;
    /// Record an outcome. Only the runner that claimed it may report.
    fn report(&self, id: i64, runner: &str, exit_code: i32, output: &str) -> Result<Job>;
    /// Mark jobs claimed but unreported past the deadline as failed. Never re-queues them.
    fn fail_abandoned(&self, older_than: i64) -> Result<Vec<Job>>;
}

pub(crate) trait SecretRepo: Send + Sync {
    /// Get latest secret with atomic lazy cleanup
    fn get_secret(&self, key: &str) -> Result<Secret>;
    /// Get specific version secret with atomic lazy cleanup
    fn get_secret_by_version(&self, key: &str, version: i32) -> Result<Secret>;
    /// A `pending` version is stored enveloped like any other but excluded from every read, so
    /// a rotation's new value exists durably without being usable until its grant succeeds.
    fn create_new_version(
        &self,
        key: &str,
        value: &SecretValue,
        master_key: MasterKey,
        ttl: Option<i64>,
        pending: bool,
    ) -> Result<Secret>;
    /// Read a pending version. The only path that sees one — every other read excludes them.
    fn get_pending(&self, key: &str, version: i32) -> Result<Secret>;
    fn commit_pending(&self, key: &str, version: i32) -> Result<()>;
    fn discard_pending(&self, key: &str, version: i32) -> Result<()>;
    fn replace_pending_value(
        &self,
        key: &str,
        version: i32,
        value: &str,
        master_key: MasterKey,
    ) -> Result<()>;
    fn delete_secret_by_version(&self, key: &str, version: i32) -> Result<()>;

    /// Rekey every secret under `old_master_key_id`, atomically. Returns the keys of secrets
    /// that could not be rekeyed; if any fail, nothing is committed.
    fn rekey_secrets(
        &self,
        old_master_key_id: &Uuid,
        old_private_key: &PrivateMasterKey,
        new_master_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Vec<String>>;
    /// Batch delete all expired secrets and return the count of deleted records.
    fn cleanup_expired_secrets(&self) -> Result<usize>;
    /// List all secrets with basic information (key, latest version, timestamps)
    fn list_secrets(&self) -> Result<Vec<SecretInfo>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MasterKeyStatus {
    Active,
    Retired,
    Disabled,
}
impl ToSql for MasterKeyStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            MasterKeyStatus::Active => Ok(rusqlite::types::ToSqlOutput::from("Active")),
            MasterKeyStatus::Retired => Ok(rusqlite::types::ToSqlOutput::from("Retired")),
            MasterKeyStatus::Disabled => Ok(rusqlite::types::ToSqlOutput::from("Disabled")),
        }
    }
}
impl FromSql for MasterKeyStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str() {
            Ok("Active") => Ok(MasterKeyStatus::Active),
            Ok("Retired") => Ok(MasterKeyStatus::Retired),
            Ok("Disabled") => Ok(MasterKeyStatus::Disabled),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

/// MasterKey struct, represents a row in the master_keys table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterKey {
    pub id: Uuid,                    // Unique identifier (e.g., UUID)
    pub public_key: String,          // Public key (PEM format)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub status: MasterKeyStatus,     // Status: Active/Retired/Disabled
    pub description: Option<String>, // Optional description
    pub metadata: Option<String>,    // Optional metadata
    /// Whether the server holds this key's private half. Secrets encrypted under a key with
    /// `server_held = false` are *cold*: the server cannot decrypt them under any
    /// circumstances, including rekey (ADR 0001).
    pub server_held: bool,
}

impl MasterKey {
    /// A key registered by a client: the server has only the public half, so secrets under it
    /// are cold.
    pub(crate) fn new(public_key: String) -> Result<Self> {
        Self::with_server_held(public_key, false)
    }

    /// A key whose private half the server holds, making secrets under it broker-serviceable.
    pub(crate) fn server_held(public_key: String) -> Result<Self> {
        Self::with_server_held(public_key, true)
    }

    fn with_server_held(public_key: String, server_held: bool) -> Result<Self> {
        Ok(MasterKey {
            id: Uuid::new_v4(),
            public_key,
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            status: MasterKeyStatus::Active,
            description: None,
            metadata: None,
            server_held,
        })
    }
}

/// MasterKeyRepo trait for managing master_keys table
pub(crate) trait MasterKeyRepo: Send + Sync {
    fn create_master_key(&self, key: &MasterKey) -> Result<()>;
    fn fetch_all_master_keys(&self) -> Result<Vec<MasterKey>>;

    /// Fetch a master key by id, including whether the server holds its private half.
    fn fetch_master_key(&self, master_key_id: &Uuid) -> Result<Option<MasterKey>>;

    /// The current key new secrets are encrypted under. Always server-held.
    fn get_valid_master_key(&self) -> Result<MasterKey>;

    /// Register one of the server's own master keys if not already present, and set its status.
    /// Idempotent, and matched on the public key so a restart does not create a duplicate.
    fn ensure_server_held(
        &self,
        public_key_pem: &str,
        status: MasterKeyStatus,
    ) -> Result<MasterKey>;
}

pub(crate) trait HealthRepo: Send + Sync {
    fn check_health(&self) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::master_key::generate_key_pair;

    #[test]
    fn test_master_key_new() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem.clone()).expect("Should create master key");

        assert_eq!(master_key.public_key, public_pem);
        assert!(matches!(master_key.status, MasterKeyStatus::Active));
        assert!(master_key.description.is_none());
        assert!(master_key.metadata.is_none());
        assert!(master_key.created_at > 0);
    }

    #[test]
    fn test_master_key_status_serialization() {
        // Test ToSql conversion
        let _active_sql = MasterKeyStatus::Active
            .to_sql()
            .expect("Should convert to SQL");
        let _retired_sql = MasterKeyStatus::Retired
            .to_sql()
            .expect("Should convert to SQL");
        let _disabled_sql = MasterKeyStatus::Disabled
            .to_sql()
            .expect("Should convert to SQL");

        // Just test that conversion works without errors
        // Test placeholder - functionality verified by other tests
    }

    #[test]
    fn test_secret_new() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret_key = "test-secret";
        let secret_data = "This is secret data";
        let version = 1;
        let ttl = Some(3600); // 1 hour

        let secret = Secret::new(secret_key, secret_data, master_key.clone(), version, ttl)
            .expect("Should create secret");

        assert_eq!(secret.key, secret_key);
        assert_eq!(secret.version, version);
        assert_eq!(secret.master_key_id, master_key.id);
        assert!(secret.expires_at.is_some());
        assert!(secret.created_at > 0);
        assert_eq!(secret.created_at, secret.updated_at);
        assert!(!secret.encrypted_data.is_empty());
        assert!(!secret.encrypted_data_key.is_empty());
        assert!(secret.metadata.is_none());
    }

    #[test]
    fn test_secret_new_without_ttl() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret = Secret::new("test-key", "test-data", master_key, 1, None)
            .expect("Should create secret");

        assert!(secret.expires_at.is_none());
    }

    #[test]
    fn test_secret_encryption_is_different() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret_data = "Same secret data";

        let secret1 = Secret::new("key1", secret_data, master_key.clone(), 1, None)
            .expect("Should create first secret");
        let secret2 = Secret::new("key2", secret_data, master_key, 2, None)
            .expect("Should create second secret");

        // Even with same data, encrypted results should be different due to random data keys
        assert_ne!(secret1.encrypted_data, secret2.encrypted_data);
        assert_ne!(secret1.encrypted_data_key, secret2.encrypted_data_key);
    }

    #[test]
    fn test_secret_rekey() {
        let (old_private_pem, old_public_pem) =
            generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");

        let old_master_key = MasterKey::new(old_public_pem).expect("Should create old master key");
        let new_master_key = MasterKey::new(new_public_pem).expect("Should create new master key");

        let original_secret =
            Secret::new("test-key", "secret-data", old_master_key.clone(), 1, None)
                .expect("Should create secret");

        let original_created_at = original_secret.created_at;
        let original_encrypted_data = original_secret.encrypted_data.clone();
        let original_encrypted_data_key = original_secret.encrypted_data_key.clone();

        let rekeyed_secret = original_secret
            .rekey(
                &old_master_key.id,
                &PrivateMasterKey::from_str(&old_private_pem).expect("Should parse"),
                &new_master_key.id,
                &new_master_key.public_key,
            )
            .expect("Should rekey");

        // Rekeying should update master key ID and encrypted data key
        assert_eq!(rekeyed_secret.master_key_id, new_master_key.id);
        assert_ne!(
            rekeyed_secret.encrypted_data_key,
            original_encrypted_data_key
        );
        assert_eq!(rekeyed_secret.encrypted_data, original_encrypted_data); // Data itself unchanged
        assert!(rekeyed_secret.updated_at >= original_created_at);
    }

    #[test]
    fn test_secret_rekey_same_key() {
        let (private_pem, public_pem) = generate_key_pair().expect("Should generate key pair");
        let private_key =
            PrivateMasterKey::from_str(&private_pem).expect("Should parse private key");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let original_secret = Secret::new("test-key", "secret-data", master_key.clone(), 1, None)
            .expect("Should create secret");

        // Rekeying to the same key should return the secret unchanged
        let rekeyed_secret = original_secret
            .clone()
            .rekey(
                &master_key.id,
                &private_key,
                &master_key.id,
                &master_key.public_key,
            )
            .expect("Should handle same rekeying");

        assert_eq!(rekeyed_secret.master_key_id, original_secret.master_key_id);
        assert_eq!(
            rekeyed_secret.encrypted_data_key,
            original_secret.encrypted_data_key
        );
    }

    #[test]
    fn test_secret_rekey_wrong_old_key() {
        let (old_private_pem, old_public_pem) =
            generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");
        let (_, wrong_public_pem) = generate_key_pair().expect("Should generate wrong key pair");

        let old_master_key = MasterKey::new(old_public_pem).expect("Should create old master key");
        let new_master_key = MasterKey::new(new_public_pem).expect("Should create new master key");
        let wrong_master_key =
            MasterKey::new(wrong_public_pem).expect("Should create wrong master key");

        let original_secret = Secret::new("test-key", "secret-data", old_master_key, 1, None)
            .expect("Should create secret");

        // Trying to rekey with wrong old key ID should fail
        let result = original_secret.rekey(
            &wrong_master_key.id, // Wrong old key ID
            &PrivateMasterKey::from_str(&old_private_pem).expect("Should parse"),
            &new_master_key.id,
            &new_master_key.public_key,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::MasterKeyMismatch(_, _, _) => {} // Expected
            _ => panic!("Expected MasterKeyMismatch error"),
        }
    }

    #[test]
    fn test_secret_rekey_with_a_private_key_that_does_not_match() {
        // A malformed private key is no longer expressible: `rekey` takes a parsed
        // `PrivateMasterKey`, so the parse either succeeded before this point or there is
        // nothing to call. What remains possible is a well-formed key that is simply the
        // wrong one.
        let (_, old_public_pem) = generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");
        let (unrelated_pem, _) = generate_key_pair().expect("Should generate unrelated pair");
        let unrelated_key =
            PrivateMasterKey::from_str(&unrelated_pem).expect("Should parse private key");

        let old_master_key = MasterKey::new(old_public_pem).expect("Should create old master key");
        let new_master_key = MasterKey::new(new_public_pem).expect("Should create new master key");

        let original_secret =
            Secret::new("test-key", "secret-data", old_master_key.clone(), 1, None)
                .expect("Should create secret");

        let result = original_secret.rekey(
            &old_master_key.id,
            &unrelated_key,
            &new_master_key.id,
            &new_master_key.public_key,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_ttl_calculation() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let ttl_seconds = 7200i64; // 2 hours
        let secret = Secret::new("test-key", "test-data", master_key, 1, Some(ttl_seconds))
            .expect("Should create secret");

        let expected_expiry = secret.created_at + ttl_seconds;
        assert_eq!(secret.expires_at, Some(expected_expiry));
    }

    #[test]
    fn test_generated_password_avoids_ambiguous_characters() {
        let spec = GenerateSpec {
            kind: GenerateKind::Password,
            length: Some(256),
        };
        let value = spec.generate().expect("Should generate");

        assert_eq!(value.chars().count(), 256);
        for c in ['0', 'O', '1', 'l', 'I'] {
            assert!(
                !value.contains(c),
                "'{c}' is confused when a value is read aloud or retyped: {value}"
            );
        }
        assert!(
            value.chars().all(|c| c.is_ascii_alphanumeric()),
            "no punctuation: symbols cost more in quoting and escaping than they add in entropy"
        );
    }

    #[test]
    fn test_generation_enforces_its_minimum_and_default() {
        let too_short = GenerateSpec {
            kind: GenerateKind::Password,
            length: Some(MIN_GENERATED_LENGTH - 1),
        };
        let err = too_short.generate().unwrap_err().to_string();
        assert!(
            err.contains(&MIN_GENERATED_LENGTH.to_string()),
            "the error must name the minimum: {err}"
        );

        let default = GenerateSpec {
            kind: GenerateKind::Password,
            length: None,
        };
        assert_eq!(
            default.generate().unwrap().chars().count(),
            DEFAULT_GENERATED_LENGTH
        );
    }

    #[test]
    fn test_generated_hex_is_hex_of_the_requested_byte_count() {
        let spec = GenerateSpec {
            kind: GenerateKind::Hex,
            length: Some(32),
        };
        let value = spec.generate().expect("Should generate");
        assert_eq!(value.len(), 64, "32 bytes render as 64 hex characters");
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_two_generations_differ() {
        let spec = GenerateSpec {
            kind: GenerateKind::Password,
            length: Some(32),
        };
        assert_ne!(spec.generate().unwrap(), spec.generate().unwrap());
    }
}
