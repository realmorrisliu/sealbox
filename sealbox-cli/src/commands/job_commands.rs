use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;

use crate::{config::Config, output::OutputManager};

/// How long to wait for a job before giving up on watching it. The job itself is unaffected —
/// it is still queued or running, and `sealbox audit` will show what became of it.
const WAIT_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(300);

/// Submit a job and wait for it.
///
/// A caller supplies a grant name and parameters — never a command (ADR 0003). What comes back
/// is an exit status and whatever the implementation printed; never a secret value.
pub async fn run(
    config: &Config,
    output: &OutputManager,
    grant: String,
    params: Vec<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let params = parse_params(&params)?;
    let client = Client::new();

    let response = client
        .post(format!("{}/v1/jobs", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&serde_json::json!({ "grant": grant, "params": params }))
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }

    let job: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse server response")?;
    let id = job["id"].as_i64().context("Server returned no job id")?;
    output.print_info(&format!(
        "Job {id} queued for runner '{}'",
        job["runner"].as_str().unwrap_or("?")
    ));

    let deadline = std::time::Instant::now() + WAIT_TIMEOUT;
    loop {
        let job = fetch(&client, config, id).await?;
        let status = job["status"].as_str().unwrap_or("");

        if status == "Succeeded" || status == "Failed" {
            if let Some(out) = job["output"].as_str()
                && !out.is_empty()
            {
                println!("{out}");
            }
            let code = job["exit_code"].as_i64().unwrap_or(-1);
            if status == "Succeeded" {
                output.print_success(&format!("Job {id} succeeded"));
                return Ok(());
            }
            anyhow::bail!("Job {id} failed (exit {code})");
        }

        if std::time::Instant::now() >= deadline {
            anyhow::bail!(
                "Stopped waiting for job {id} after {}s. It is still {status} — check \
                 `sealbox-cli audit` for what became of it.",
                WAIT_TIMEOUT.as_secs()
            );
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn fetch(client: &Client, config: &Config, id: i64) -> Result<serde_json::Value> {
    let response = client
        .get(format!("{}/v1/jobs/{id}", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }
    response.json().await.context("Failed to parse job")
}

fn parse_params(raw: &[String]) -> Result<BTreeMap<String, String>> {
    raw.iter()
        .map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .with_context(|| format!("Parameter '{pair}' is not key=value"))
        })
        .collect()
}

/// Rotate a secret through a grant.
///
/// The new value is generated on the server and is never displayed — saying so matters, because
/// a caller may otherwise sit waiting for it.
pub async fn rotate(
    config: &Config,
    output: &OutputManager,
    secret: String,
    via: String,
    from_output: bool,
    params: Vec<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let params = parse_params(&params)?;
    let client = Client::new();

    let response = client
        .post(format!("{}/v1/rotate/{secret}", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&serde_json::json!({
            "via": via,
            "from_output": from_output,
            "params": params,
        }))
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }

    let started: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse server response")?;
    let id = started["job"]
        .as_i64()
        .context("Server returned no job id")?;
    output.print_info(&format!(
        "Rotating '{secret}' through '{via}' (job {id}). The new value was generated on the \
         server and is not shown to anyone."
    ));

    let deadline = std::time::Instant::now() + WAIT_TIMEOUT;
    loop {
        let job = fetch(&client, config, id).await?;
        match job["status"].as_str().unwrap_or("") {
            "Succeeded" => {
                if let Some(out) = job["output"].as_str()
                    && !out.is_empty()
                {
                    println!("{out}");
                }
                output.print_success(&format!(
                    "'{secret}' rotated. Version {} is now current.",
                    started["pending_version"]
                ));
                return Ok(());
            }
            "Failed" => {
                if let Some(out) = job["output"].as_str()
                    && !out.is_empty()
                {
                    println!("{out}");
                }
                // The distinction that matters: nothing changed, so whatever is deployed still
                // works.
                anyhow::bail!(
                    "Rotation of '{secret}' failed. The previous value is still current and \
                     unchanged — nothing upstream was left disagreeing with what is stored."
                );
            }
            status => {
                if std::time::Instant::now() >= deadline {
                    anyhow::bail!(
                        "Stopped waiting for the rotation of '{secret}' after {}s; it is still \
                         {status}. Check `sealbox-cli audit` for what became of it.",
                        WAIT_TIMEOUT.as_secs()
                    );
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_key_value_pairs() {
        let parsed = parse_params(&["ns=prod".into(), "url=https://a?b=c".into()]).unwrap();
        assert_eq!(parsed["ns"], "prod");
        // Only the first `=` separates, so a value may contain more.
        assert_eq!(parsed["url"], "https://a?b=c");
        assert!(parse_params(&["nope".into()]).is_err());
    }
}
