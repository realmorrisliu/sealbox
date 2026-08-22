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
    /// The role that creates objects in this database — the one migrations run as.
    ///
    /// Required, and deliberately not inferred from the connecting account. Default privileges
    /// attach to whoever *creates* an object, so Postgres can only record "when `owner` creates a
    /// table, grant to this role". Guessing wrong is silent: the role is provisioned, everything
    /// reports success, and it cannot see a single table anyone migrates in afterwards.
    pub owner: String,
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
            require_non_empty(adapter, "owner", &parsed.owner)?;

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
            // Both reach SQL as identifiers, so they may only look like identifiers.
            for (field, value) in [
                ("role_prefix", &parsed.role_prefix),
                ("owner", &parsed.owner),
            ] {
                if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(SealboxError::InvalidRequest(format!(
                        "adapter `{adapter}`: {field} may contain only letters, digits, and \
                         underscores"
                    )));
                }
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
            "host": "db", "database": "app", "role_prefix": "app", "owner": "migrator",
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
            "role_prefix": "app\"; DROP TABLE users; --", "owner": "migrator",
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
                    "host": "db.internal", "database": "app", "role_prefix": "app", "owner": "migrator",
                    "privileges": ["CONNECT", "SELECT", "INSERT"],
                })
            )
            .is_ok()
        );
    }
}

/// The worked examples are the template library: an agent asked for a grant it has not written
/// before reads them and imitates one. So they have to be correct in the way a grant is correct,
/// not merely plausible — and that is what these check.
///
/// They earn their place. Four of the seven were **not valid TOML** when this was written: a
/// multi-line `config = { … }` is an inline table spread over several lines, which the spec does
/// not allow. Two more parsed and were still wrong, because moving `config` into a sub-table
/// silently swallowed the `secrets` and `then` lines that followed it — the declaration a human
/// is supposed to read, quietly absorbed into the configuration.
#[cfg(test)]
mod examples {
    use super::*;

    fn examples() -> Vec<(String, toml::Value)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("examples/grants");

        let mut found = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("examples/grants should exist") {
            let path = entry.expect("Should read the entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a file name")
                .to_string();
            let text = std::fs::read_to_string(&path).expect("Should read the example");
            let parsed: toml::Value =
                toml::from_str(&text).unwrap_or_else(|e| panic!("{name} is not valid TOML: {e}"));
            found.push((name, parsed));
        }
        assert!(!found.is_empty(), "there should be examples to check");
        found
    }

    #[test]
    fn every_example_is_a_grant() {
        for (file, parsed) in examples() {
            let table = parsed.as_table().expect("a table");
            assert_eq!(table.len(), 1, "{file}: one grant per file");

            let (name, body) = table.iter().next().expect("the grant");
            let body = body
                .as_table()
                .unwrap_or_else(|| panic!("{file}: `{name}` should be a table"));

            assert!(body.contains_key("runner"), "{file}: needs a runner");
            assert!(
                body.contains_key("secrets") || body.contains_key("files"),
                "{file}: a grant declares what it may use"
            );
            assert!(
                body.contains_key("adapter") != body.contains_key("script"),
                "{file}: exactly one of `adapter` or `script` — `{name}` has {}",
                if body.contains_key("adapter") {
                    "both"
                } else {
                    "neither"
                }
            );
        }
    }

    #[test]
    fn every_adapter_example_would_be_accepted_at_approval() {
        for (file, parsed) in examples() {
            let (name, body) = parsed
                .as_table()
                .and_then(|t| t.iter().next())
                .expect("the grant");
            let body = body.as_table().expect("a table");

            let Some(adapter) = body.get("adapter").and_then(|a| a.as_str()) else {
                continue;
            };
            let config = body
                .get("config")
                .cloned()
                .unwrap_or(toml::Value::Table(Default::default()));
            let config: serde_json::Value =
                serde_json::to_value(config).expect("Should convert to JSON");

            // The same call `grant add` makes. An example that would be refused is worse than no
            // example: it is a template that produces a refusal.
            validate_config(adapter, &config)
                .unwrap_or_else(|e| panic!("{file}: `{name}` would be refused: {e}"));
        }
    }

    #[test]
    fn no_example_parameterises_a_secret_name() {
        // The rule is enforced server-side, but an example carrying one would teach the shape.
        for (file, parsed) in examples() {
            let body = parsed
                .as_table()
                .and_then(|t| t.values().next())
                .and_then(|v| v.as_table())
                .expect("the grant");

            for source in ["secrets", "files"] {
                let Some(map) = body.get(source).and_then(|s| s.as_table()) else {
                    continue;
                };
                for (injected, secret) in map {
                    let secret = secret.as_str().unwrap_or_default();
                    assert!(
                        !secret.contains('{'),
                        "{file}: `{injected}` names `{secret}`, and a parameter there would let \
                         whoever invokes the grant choose which credential it reaches"
                    );
                }
            }
        }
    }
}
