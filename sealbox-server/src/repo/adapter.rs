//! Adapter configuration: typed, closed, and validated when a human is present.
//!
//! An adapter is worth having because a script can do anything its declared secrets permit and
//! an adapter cannot — a script holding a kubeconfig could `delete ns prod`, while
//! `kubernetes-secret` can write one Secret and nothing else (ADR 0007).
//!
//! That property lives here. Each adapter's settings are a struct that rejects unknown fields,
//! and nothing in one can carry a command, a query, a resource kind, or a verb. For a field to
//! make an adapter do something else, someone would have to add it below — which is a code
//! change with a review attached, rather than a configuration value nobody looked at.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SealboxError};

/// Privileges `postgres-role` may grant.
///
/// Closed not because the others are dangerous in themselves, but because an open set would have
/// to be interpolated into SQL — and a field interpolated into SQL is a field that can carry SQL.
/// These are matched against constants and never concatenated from input. Anything beyond them
/// is a script.
pub const POSTGRES_PRIVILEGES: &[&str] =
    &["CONNECT", "SELECT", "INSERT", "UPDATE", "DELETE", "USAGE"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KubernetesSecretConfig {
    /// The namespace to write into. Becomes a `--namespace` argument and nothing else.
    pub namespace: String,
    /// The Secret's name.
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PostgresRoleConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database: String,
    /// Roles are named `<prefix>_<serial>`. A prefix rather than a full name, so the grant stays
    /// stable across rotations and can be approved once — grants are immutable, so a configured
    /// role name would mean a new grant for every rotation.
    pub role_prefix: String,
    /// Drawn from `POSTGRES_PRIVILEGES`.
    pub privileges: Vec<String>,
    /// The schema privileges apply to.
    #[serde(default = "default_schema")]
    pub schema: String,
}

fn default_port() -> u16 {
    5432
}

fn default_schema() -> String {
    "public".to_string()
}

/// Check an adapter's configuration while a human is looking at the grant.
///
/// A namespace that does not parse, or a privilege nobody recognises, is much better discovered
/// at approval than at three in the morning when a rotation fails halfway.
pub fn validate_config(adapter: &str, config: &serde_json::Value) -> Result<()> {
    match adapter {
        "kubernetes-secret" => {
            let parsed: KubernetesSecretConfig = parse(adapter, config)?;
            require_non_empty(adapter, "namespace", &parsed.namespace)?;
            require_non_empty(adapter, "name", &parsed.name)?;
            Ok(())
        }
        "postgres-role" => {
            let parsed: PostgresRoleConfig = parse(adapter, config)?;
            require_non_empty(adapter, "host", &parsed.host)?;
            require_non_empty(adapter, "database", &parsed.database)?;
            require_non_empty(adapter, "role_prefix", &parsed.role_prefix)?;

            if parsed.privileges.is_empty() {
                return Err(SealboxError::InvalidRequest(format!(
                    "adapter `{adapter}` needs at least one privilege. Permitted: {}",
                    POSTGRES_PRIVILEGES.join(", ")
                )));
            }
            for privilege in &parsed.privileges {
                if !POSTGRES_PRIVILEGES.contains(&privilege.to_uppercase().as_str()) {
                    return Err(SealboxError::InvalidRequest(format!(
                        "adapter `{adapter}` does not permit privilege `{privilege}`. \
                         Permitted: {}. Anything beyond these is a script.",
                        POSTGRES_PRIVILEGES.join(", ")
                    )));
                }
            }
            // A role prefix reaches SQL, so it may only look like an identifier.
            if !parsed
                .role_prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(SealboxError::InvalidRequest(format!(
                    "adapter `{adapter}`: role_prefix may contain only letters, digits, and \
                     underscores"
                )));
            }
            Ok(())
        }
        other => Err(SealboxError::InvalidRequest(format!(
            "unknown adapter `{other}`"
        ))),
    }
}

fn parse<T: serde::de::DeserializeOwned>(adapter: &str, config: &serde_json::Value) -> Result<T> {
    serde_json::from_value(config.clone()).map_err(|e| {
        SealboxError::InvalidRequest(format!("adapter `{adapter}` configuration: {e}"))
    })
}

fn require_non_empty(adapter: &str, field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(SealboxError::InvalidRequest(format!(
            "adapter `{adapter}` requires `{field}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_configuration_fields_are_refused() {
        // A typo in `namespace` that silently wrote to `default` would be found in production,
        // by someone who did not make it.
        let config = json!({ "namespace": "prod", "name": "app", "namspace": "typo" });
        let err = validate_config("kubernetes-secret", &config)
            .unwrap_err()
            .to_string();
        assert!(err.contains("namspace"), "{err}");
    }

    #[test]
    fn a_privilege_outside_the_closed_set_is_refused() {
        let config = json!({
            "host": "db", "database": "app", "role_prefix": "app",
            "privileges": ["SELECT", "DROP"],
        });
        let err = validate_config("postgres-role", &config)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("DROP"),
            "the offending privilege is named: {err}"
        );
        assert!(
            err.contains("CONNECT"),
            "and the permitted ones listed: {err}"
        );
    }

    #[test]
    fn a_missing_required_setting_is_named() {
        let err = validate_config("kubernetes-secret", &json!({ "namespace": "prod" }))
            .unwrap_err()
            .to_string();
        assert!(err.contains("name"), "{err}");
    }

    #[test]
    fn a_role_prefix_that_could_carry_sql_is_refused() {
        // The prefix reaches SQL as an identifier, so it may only look like one.
        let config = json!({
            "host": "db", "database": "app",
            "role_prefix": "app\"; DROP TABLE users; --",
            "privileges": ["CONNECT"],
        });
        assert!(validate_config("postgres-role", &config).is_err());
    }

    #[test]
    fn a_valid_configuration_passes() {
        assert!(
            validate_config(
                "kubernetes-secret",
                &json!({ "namespace": "prod", "name": "app-secrets" })
            )
            .is_ok()
        );
        assert!(
            validate_config(
                "postgres-role",
                &json!({
                    "host": "db.internal", "database": "app", "role_prefix": "app",
                    "privileges": ["CONNECT", "SELECT", "INSERT"],
                })
            )
            .is_ok()
        );
    }
}
