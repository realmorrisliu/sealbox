//! Registering the platforms whose signatures authenticate a workload.
//!
//! What is uploaded is public key material — a JWKS is published by definition — so this widens
//! authority without handing anything over. It is an admin operation because of the first half:
//! registering an issuer says identities from that platform may act here.

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::{IssuerCommands, config::Config, output::OutputManager};

pub async fn handle_command(command: IssuerCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());
    match command {
        IssuerCommands::Add {
            name,
            issuer_url,
            jwks_file,
        } => add(config, &output, name, issuer_url, jwks_file).await,
        IssuerCommands::Update { name, jwks_file } => {
            update(config, &output, name, jwks_file).await
        }
        IssuerCommands::List => list(config, &output).await,
        IssuerCommands::Rm { name } => remove(config, &output, name).await,
    }
}

async fn add(
    config: &Config,
    output: &OutputManager,
    name: String,
    url: String,
    jwks_file: String,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let jwks = read_jwks(&jwks_file)?;
    let result = send(
        config,
        Client::new()
            .post(format!("{}/v1/issuers", config.server.url))
            .json(&json!({ "name": name, "url": url, "jwks": jwks })),
    )
    .await?;

    output.print_success(&format!("Issuer '{name}' registered."));
    output.print_value(&result)?;
    output.print_info(
        "Bind a runner to it: identity create <name> --role runner --issuer <issuer> \
         --subject <sub> --audience <aud>",
    );
    Ok(())
}

async fn update(
    config: &Config,
    output: &OutputManager,
    name: String,
    jwks_file: String,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let jwks = read_jwks(&jwks_file)?;
    let result = send(
        config,
        Client::new()
            .put(format!("{}/v1/issuers/{name}", config.server.url))
            .json(&json!({ "jwks": jwks })),
    )
    .await?;

    output.print_success(&format!("Issuer '{name}' now holds the supplied keys."));
    output.print_value(&result)?;
    output.print_info(
        "During a rotation, register both keys at once and remove the old one only when nothing \
         presents it any more.",
    );
    Ok(())
}

async fn list(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;
    let result = send(
        config,
        Client::new().get(format!("{}/v1/issuers", config.server.url)),
    )
    .await?;
    output.print_value(&result)?;
    Ok(())
}

async fn remove(config: &Config, output: &OutputManager, name: String) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;
    send(
        config,
        Client::new().delete(format!("{}/v1/issuers/{name}", config.server.url)),
    )
    .await?;
    output.print_success(&format!(
        "Issuer '{name}' removed. Every identity bound to it has stopped authenticating."
    ));
    Ok(())
}

/// Read and sanity-check locally, so an obvious mistake fails with the file in hand rather than
/// as a server error about a request body.
fn read_jwks(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read the JWKS at {path}"))?;
    serde_json::from_str::<serde_json::Value>(&content)
        .with_context(|| format!("{path} is not JSON. It should be the issuer's JWKS document."))?;
    Ok(content)
}

async fn send(config: &Config, request: reqwest::RequestBuilder) -> Result<serde_json::Value> {
    let response = request
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("Server returned {status}: {body}");
    }
    Ok(serde_json::from_str(&body).unwrap_or(serde_json::Value::Null))
}
