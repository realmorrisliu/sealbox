use std::collections::BTreeMap;

use anyhow::{Context, Result};
use reqwest::Client;

use crate::{GrantCommands, config::Config, output::OutputManager};

pub async fn handle_command(command: GrantCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());
    match command {
        GrantCommands::Add { file } => add(config, &output, file).await,
        GrantCommands::List => list(config, &output).await,
        GrantCommands::Show { name } => show(config, &output, name).await,
        GrantCommands::Rm { name } => remove(config, &output, name).await,
    }
}

/// Parse the file locally, so a malformed one fails here with the file in hand rather than as a
/// server error about a request body.
///
/// The TOML top level is a table per grant, which is how the examples read. A file may hold
/// several; each is submitted and approved on its own.
async fn add(config: &Config, output: &OutputManager, file: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let content = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read grant file: {file}"))?;
    let parsed: BTreeMap<String, toml::Value> =
        toml::from_str(&content).with_context(|| format!("Failed to parse TOML in {file}"))?;

    if parsed.is_empty() {
        anyhow::bail!("{file} defines no grants");
    }

    for (name, body) in parsed {
        let mut payload: serde_json::Value = serde_json::to_value(&body)
            .with_context(|| format!("Grant '{name}' could not be converted to JSON"))?;
        payload["name"] = serde_json::json!(name);

        // Show what is actually being approved. The secrets line is the security-relevant part:
        // sealbox confines the implementation to exactly these, so however it is written it
        // cannot reach anything else.
        output.print_info(&format!("Approving grant '{name}':"));
        print_declaration(output, &payload);

        let response = Client::new()
            .post(format!("{}/v1/grants", config.server.url))
            .bearer_auth(&config.server.token)
            .json(&payload)
            .send()
            .await
            .context("Failed to request server")?;

        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            anyhow::bail!("Grant '{name}' was refused ({status}): {detail}");
        }
        output.print_success(&format!("Grant '{name}' approved."));
    }
    Ok(())
}

fn print_declaration(output: &OutputManager, payload: &serde_json::Value) {
    if let Some(secrets) = payload.get("secrets").and_then(|s| s.as_object()) {
        if secrets.is_empty() {
            output.print_info("  secrets: none");
        } else {
            output.print_info("  secrets it may use:");
            for (injected, secret) in secrets {
                let secret = secret.as_str().unwrap_or_default();
                output.print_info(&format!("    {secret}  (as {injected})"));
            }
        }
    }
    if let Some(adapter) = payload.get("adapter").and_then(|a| a.as_str()) {
        output.print_info(&format!("  adapter: {adapter}"));
    }
    if payload.get("script").is_some() {
        output.print_info("  script: a custom script (can do anything its secrets permit)");
    }
    if let Some(runner) = payload.get("runner").and_then(|r| r.as_str()) {
        output.print_info(&format!("  runs on: {runner}"));
    }
    if let Some(then) = payload.get("then").and_then(|t| t.as_array())
        && !then.is_empty()
    {
        let names: Vec<_> = then.iter().filter_map(|v| v.as_str()).collect();
        output.print_info(&format!("  then: {}", names.join(" → ")));
    }
}

async fn list(config: &Config, output: &OutputManager) -> Result<()> {
    output.print_value(&fetch(config, "/v1/grants").await?)?;
    Ok(())
}

async fn show(config: &Config, output: &OutputManager, name: String) -> Result<()> {
    output.print_value(&fetch(config, &format!("/v1/grants/{name}")).await?)?;
    Ok(())
}

async fn remove(config: &Config, output: &OutputManager, name: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let response = Client::new()
        .delete(format!("{}/v1/grants/{name}", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {detail}");
    }
    output.print_success(&format!(
        "Grant '{name}' removed. There is no update: to change a grant, add the replacement — \
         which puts its declaration in front of you again."
    ));
    Ok(())
}

/// The grants that may use a secret. The question no other secret manager can answer.
pub async fn uses(config: &Config, output: &OutputManager, secret: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let mut url = reqwest::Url::parse(&format!("{}/v1/secrets", config.server.url))
        .context("Invalid server URL")?;
    url.query_pairs_mut().append_pair("uses", &secret);

    let response = Client::new()
        .get(url)
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {detail}");
    }
    output.print_value(&response.json::<serde_json::Value>().await?)?;
    Ok(())
}

async fn fetch(config: &Config, path: &str) -> Result<serde_json::Value> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let response = Client::new()
        .get(format!("{}{path}", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {detail}");
    }
    response
        .json()
        .await
        .context("Failed to parse server response")
}
