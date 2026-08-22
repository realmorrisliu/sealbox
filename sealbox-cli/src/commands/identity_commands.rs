use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::{IdentityCommands, config::Config, output::OutputManager};

pub async fn handle_command(command: IdentityCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());
    match command {
        IdentityCommands::Create { name, role } => create(config, &output, name, role).await,
        IdentityCommands::List => list(config, &output).await,
        IdentityCommands::Revoke { name } => revoke(config, &output, name).await,
    }
}

async fn create(config: &Config, output: &OutputManager, name: String, role: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let response = Client::new()
        .post(format!("{}/v1/identities", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&json!({ "name": name, "role": role }))
        .send()
        .await
        .context("Failed to request server")?;

    let result = expect_json(response).await?;

    // The one and only time this token exists outside the caller's memory.
    output.print_success(&format!("Identity '{name}' created."));
    output.print_value(&result)?;
    output.print_warning(
        "The token above is shown once and cannot be retrieved. Store it now; if it is lost, \
         revoke the identity and create another.",
    );
    Ok(())
}

async fn list(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let response = Client::new()
        .get(format!("{}/v1/identities", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    output.print_value(&expect_json(response).await?)?;
    Ok(())
}

async fn revoke(config: &Config, output: &OutputManager, name: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let response = Client::new()
        .delete(format!("{}/v1/identities/{name}", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    expect_json(response).await?;
    output.print_success(&format!(
        "Identity '{name}' revoked. Its next request will be refused; every other identity is \
         unaffected."
    ));
    Ok(())
}

/// Claim a server that has no identities yet.
pub async fn bootstrap(
    config: &Config,
    output: &OutputManager,
    token: String,
    name: String,
) -> Result<()> {
    let response = Client::new()
        .post(format!("{}/v1/bootstrap", config.server.url))
        .json(&json!({ "token": token, "name": name }))
        .send()
        .await
        .context("Failed to request server")?;

    let result = expect_json(response).await.context(
        "Bootstrap was refused. It is accepted only while the server has no identities, only \
         with the token it was started with, and only within the bootstrap window",
    )?;

    output.print_success(&format!("Admin identity '{name}' created."));
    output.print_value(&result)?;
    output.print_warning(
        "Open the enrolment link and register your passkey, then unset SEALBOX_BOOTSTRAP_TOKEN \
         on the server — it has served its \
         purpose and only widens exposure from here.",
    );
    Ok(())
}

async fn expect_json(response: reqwest::Response) -> Result<serde_json::Value> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!("Server returned {status}: {body}");
    }
    response
        .json()
        .await
        .context("Failed to parse server response")
}
