use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngExt;
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{
        ApiTokenMetadata, AuthenticatedTenant, IssuedApiToken, Tenant, TenantRepo, TenantStatus,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteTenantRepo;

impl SqliteTenantRepo {
    pub fn init_tables(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tenants (
                id TEXT PRIMARY KEY,
                display_name TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS api_tokens (
                id BLOB PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                token_hash BLOB NOT NULL,
                label TEXT,
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                revoked_at INTEGER,
                last_used_at INTEGER,
                FOREIGN KEY (tenant_id) REFERENCES tenants(id)
            );
            CREATE INDEX IF NOT EXISTS idx_api_tokens_tenant
                ON api_tokens(tenant_id, created_at);",
        )?;
        Ok(())
    }

    fn now() -> i64 {
        time::OffsetDateTime::now_utc().unix_timestamp()
    }

    fn tenant_id() -> String {
        format!("ten_{}", Uuid::new_v4().simple())
    }

    fn token_hash(token: &str) -> Vec<u8> {
        Sha256::digest(token.as_bytes()).to_vec()
    }

    fn token_id(token: &str) -> Option<Uuid> {
        let value = token.strip_prefix("sbx_t_")?;
        let (id, secret) = value.split_once('.')?;
        if secret.is_empty() {
            return None;
        }
        Uuid::parse_str(id).ok()
    }

    fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
        let max_len = left.len().max(right.len());
        let mut diff = left.len() ^ right.len();
        for index in 0..max_len {
            let left_byte = left.get(index).copied().unwrap_or(0);
            let right_byte = right.get(index).copied().unwrap_or(0);
            diff |= usize::from(left_byte ^ right_byte);
        }
        diff == 0
    }

    fn row_to_tenant(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tenant> {
        Ok(Tenant {
            id: row.get(0)?,
            display_name: row.get(1)?,
            status: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    fn row_to_token(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiTokenMetadata> {
        Ok(ApiTokenMetadata {
            id: row.get(0)?,
            tenant_id: row.get(1)?,
            label: row.get(2)?,
            created_at: row.get(3)?,
            expires_at: row.get(4)?,
            revoked_at: row.get(5)?,
            last_used_at: row.get(6)?,
        })
    }
}

impl TenantRepo for SqliteTenantRepo {
    fn create_tenant(
        &self,
        conn: &rusqlite::Connection,
        display_name: Option<String>,
    ) -> Result<Tenant> {
        let now = Self::now();
        let tenant = Tenant {
            id: Self::tenant_id(),
            display_name,
            status: TenantStatus::Active,
            created_at: now,
            updated_at: now,
        };
        conn.execute(
            "INSERT INTO tenants (id, display_name, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &tenant.id,
                &tenant.display_name,
                &tenant.status,
                tenant.created_at,
                tenant.updated_at,
            ),
        )?;
        Ok(tenant)
    }

    fn list_tenants(&self, conn: &rusqlite::Connection) -> Result<Vec<Tenant>> {
        let mut stmt = conn.prepare(
            "SELECT id, display_name, status, created_at, updated_at
             FROM tenants ORDER BY created_at, id",
        )?;
        Ok(stmt
            .query_map([], Self::row_to_tenant)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn get_tenant(&self, conn: &rusqlite::Connection, tenant_id: &str) -> Result<Option<Tenant>> {
        Ok(conn
            .query_row(
                "SELECT id, display_name, status, created_at, updated_at
                 FROM tenants WHERE id = ?1",
                [tenant_id],
                Self::row_to_tenant,
            )
            .optional()?)
    }

    fn set_tenant_status(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
        status: TenantStatus,
    ) -> Result<Tenant> {
        let now = Self::now();
        let changed = conn.execute(
            "UPDATE tenants SET status = ?1, updated_at = ?2 WHERE id = ?3",
            (&status, now, tenant_id),
        )?;
        if changed == 0 {
            return Err(SealboxError::TenantNotFound(tenant_id.to_string()));
        }
        self.get_tenant(conn, tenant_id)?
            .ok_or_else(|| SealboxError::TenantNotFound(tenant_id.to_string()))
    }

    fn issue_token(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
        label: Option<String>,
        expires_at: Option<i64>,
    ) -> Result<IssuedApiToken> {
        if self.get_tenant(conn, tenant_id)?.is_none() {
            return Err(SealboxError::TenantNotFound(tenant_id.to_string()));
        }
        let token_id = Uuid::new_v4();
        let mut secret = vec![0_u8; 32];
        rand::rng().fill(&mut secret[..]);
        let token = format!(
            "sbx_t_{}.{}",
            token_id.simple(),
            URL_SAFE_NO_PAD.encode(secret)
        );
        let created_at = Self::now();
        let metadata = ApiTokenMetadata {
            id: token_id,
            tenant_id: tenant_id.to_string(),
            label,
            created_at,
            expires_at,
            revoked_at: None,
            last_used_at: None,
        };
        conn.execute(
            "INSERT INTO api_tokens (
                id, tenant_id, token_hash, label, created_at, expires_at, revoked_at, last_used_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL)",
            rusqlite::params![
                metadata.id,
                metadata.tenant_id,
                Self::token_hash(&token),
                metadata.label,
                metadata.created_at,
                metadata.expires_at,
            ],
        )?;
        Ok(IssuedApiToken { metadata, token })
    }

    fn list_tokens(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
    ) -> Result<Vec<ApiTokenMetadata>> {
        if self.get_tenant(conn, tenant_id)?.is_none() {
            return Err(SealboxError::TenantNotFound(tenant_id.to_string()));
        }
        let mut stmt = conn.prepare(
            "SELECT id, tenant_id, label, created_at, expires_at, revoked_at, last_used_at
             FROM api_tokens WHERE tenant_id = ?1 ORDER BY created_at, id",
        )?;
        Ok(stmt
            .query_map([tenant_id], Self::row_to_token)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn revoke_token(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
        token_id: &Uuid,
    ) -> Result<()> {
        let changed = conn.execute(
            "UPDATE api_tokens SET revoked_at = COALESCE(revoked_at, ?1)
             WHERE tenant_id = ?2 AND id = ?3",
            rusqlite::params![Self::now(), tenant_id, token_id],
        )?;
        if changed == 0 {
            return Err(SealboxError::ApiTokenNotFound(*token_id));
        }
        Ok(())
    }

    fn authenticate_token(
        &self,
        conn: &rusqlite::Connection,
        token: &str,
    ) -> Result<Option<AuthenticatedTenant>> {
        let Some(token_id) = Self::token_id(token) else {
            return Ok(None);
        };
        let row = conn
            .query_row(
                "SELECT t.id, t.status, a.token_hash, a.expires_at, a.revoked_at
                 FROM api_tokens AS a
                 JOIN tenants AS t ON t.id = a.tenant_id
                 WHERE a.id = ?1",
                [token_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, TenantStatus>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((tenant_id, status, expected_hash, expires_at, revoked_at)) = row else {
            return Ok(None);
        };
        let now = Self::now();
        if status != TenantStatus::Active
            || revoked_at.is_some()
            || expires_at.is_some_and(|expires| expires <= now)
            || !Self::constant_time_eq(&expected_hash, &Self::token_hash(token))
        {
            return Ok(None);
        }
        conn.execute(
            "UPDATE api_tokens SET last_used_at = ?1 WHERE id = ?2",
            rusqlite::params![now, token_id],
        )?;
        Ok(Some(AuthenticatedTenant {
            tenant_id,
            token_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        SqliteTenantRepo::init_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn tokens_are_tenant_scoped_revocable_and_not_stored_in_plaintext() {
        let conn = setup();
        let repo = SqliteTenantRepo;
        let tenant = repo
            .create_tenant(&conn, Some("Example".to_string()))
            .unwrap();
        let issued = repo
            .issue_token(&conn, &tenant.id, Some("client".to_string()), None)
            .unwrap();

        let authenticated = repo
            .authenticate_token(&conn, &issued.token)
            .unwrap()
            .unwrap();
        assert_eq!(authenticated.tenant_id, tenant.id);
        assert_eq!(authenticated.token_id, issued.metadata.id);

        let stored_hash: Vec<u8> = conn
            .query_row(
                "SELECT token_hash FROM api_tokens WHERE id = ?1",
                [issued.metadata.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_ne!(stored_hash, issued.token.as_bytes());

        repo.revoke_token(&conn, &tenant.id, &issued.metadata.id)
            .unwrap();
        assert!(
            repo.authenticate_token(&conn, &issued.token)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn suspended_tenant_cannot_authenticate() {
        let conn = setup();
        let repo = SqliteTenantRepo;
        let tenant = repo.create_tenant(&conn, None).unwrap();
        let issued = repo.issue_token(&conn, &tenant.id, None, None).unwrap();
        repo.set_tenant_status(&conn, &tenant.id, TenantStatus::Suspended)
            .unwrap();

        assert!(
            repo.authenticate_token(&conn, &issued.token)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn expired_token_cannot_authenticate() {
        let conn = setup();
        let repo = SqliteTenantRepo;
        let tenant = repo.create_tenant(&conn, None).unwrap();
        let issued = repo
            .issue_token(&conn, &tenant.id, None, Some(SqliteTenantRepo::now() - 1))
            .unwrap();

        assert!(
            repo.authenticate_token(&conn, &issued.token)
                .unwrap()
                .is_none()
        );
    }
}
