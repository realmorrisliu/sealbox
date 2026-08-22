use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Value, json};

use crate::{SecretCommands, config::Config, output::OutputManager};

pub async fn handle_command(command: SecretCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        SecretCommands::Set { key, ttl } => set_secret(config, &output, key, ttl).await,
        SecretCommands::Gen {
            key,
            r#type,
            length,
            ttl,
        } => generate_secret(config, &output, key, r#type, length, ttl).await,
        SecretCommands::Show { key, version } => show_secret(config, &output, key, version).await,
        SecretCommands::Delete { key, version } => {
            delete_secret(config, &output, key, version).await
        }
        SecretCommands::List => list_secrets(config, &output).await,
        SecretCommands::Uses { key } => {
            crate::commands::grant_commands::uses(config, &output, key).await
        }
    }
}

/// Strip the trailing newline a pipe adds, and refuse an empty value.
///
/// Only the trailing newline: leading and interior whitespace can be part of a credential, and
/// silently altering a value is worse than storing an odd one.
fn clean_value(raw: String, key: &str) -> Result<String> {
    let value = raw.trim_end_matches(['\n', '\r']).to_string();
    if value.is_empty() {
        anyhow::bail!(
            "No value given for '{key}'. Pipe it in — `printf %s \"$VALUE\" | sealbox-cli secret \
             set {key}` — or run this from a terminal to be prompted."
        );
    }
    Ok(value)
}

async fn set_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    ttl: Option<i64>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    // Always stdin. There is no argument form: while one exists it gets used, and every use
    // puts a credential into shell history and into `ps` output for every user on the machine.
    let raw = if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        output.print_info("Enter secret value (input will be hidden):");
        rpassword::read_password().context("Failed to read secret value")?
    } else {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read the secret value from stdin")?;
        buffer
    };
    let secret_value = clean_value(raw, &key)?;

    // Send plaintext to server (server will handle encryption)
    output.print_info("Saving to server...");

    let payload = json!({
        "secret": secret_value,
        "ttl": ttl
    });

    let client = Client::new();
    let response = client
        .put(format!("{}/v1/secrets/{}", config.server.url, key))
        .bearer_auth(&config.server.token)
        .json(&payload)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if status.is_success() {
        let result: Value = response
            .json()
            .await
            .context("Failed to parse server response")?;

        output.print_success(&format!("Secret '{key}' saved successfully!"));
        output.print_value(&result)?;
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!(
            "Server returned error (status code: {}):\n{}",
            status,
            error_body
        );
    }

    Ok(())
}

/// Metadata only: that it exists, its version, and when it last changed.
///
/// There used to be a decryption here — the server returned the ciphertext and this fetched the
/// local master key to open it. That is gone. Nothing an agent can reach returns a value or the
/// ciphertext of one, and a **cold** secret is read offline from a copy of the database, without
/// a server, which is the only thing that works when one is actually needed.
async fn show_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    version: Option<i32>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let mut url = format!("{}/v1/secrets/{}", config.server.url, key);
    if let Some(v) = version {
        url.push_str(&format!("?version={v}"));
    }

    let response = Client::new()
        .get(&url)
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {detail}");
    }

    output.print_value(&response.json::<Value>().await?)?;
    Ok(())
}

async fn delete_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    version: i32,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let url = format!(
        "{}/v1/secrets/{}?version={}",
        config.server.url, key, version
    );

    output.print_info(&format!("Deleting secret '{key}' version {version}..."));

    let client = Client::new();
    let response = client
        .delete(&url)
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if status.is_success() {
        output.print_success(&format!(
            "Secret '{key}' version {version} deleted successfully!"
        ));
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!(
            "Server returned error (status code: {}):\n{}",
            status,
            error_body
        );
    }

    Ok(())
}

async fn list_secrets(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let response = Client::new()
        .get(format!("{}/v1/secrets", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }

    // Metadata only: keys, versions, timestamps. Never a value.
    let result: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse server response")?;
    output.print_value(&result)?;
    Ok(())
}

async fn generate_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    kind: String,
    length: Option<usize>,
    ttl: Option<i64>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let mut generate = serde_json::json!({ "type": kind });
    if let Some(length) = length {
        generate["length"] = serde_json::json!(length);
    }
    let mut payload = serde_json::json!({ "generate": generate });
    if let Some(ttl) = ttl {
        payload["ttl"] = serde_json::json!(ttl);
    }

    let response = Client::new()
        .put(format!("{}/v1/secrets/{key}", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&payload)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }

    let result: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse server response")?;

    output.print_success(&format!("Generated and stored '{key}'."));
    output.print_value(&result)?;
    // Saying so is the point: an agent can create a credential it will never be able to read.
    output.print_info("The value was generated on the server and is not returned to anyone.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_value_strips_only_the_trailing_newline() {
        // What a pipe adds.
        assert_eq!(clean_value("hunter2\n".into(), "k").unwrap(), "hunter2");
        assert_eq!(clean_value("hunter2\r\n".into(), "k").unwrap(), "hunter2");
        // What is part of the value: leading and interior whitespace both survive, because
        // silently altering a credential is worse than storing an odd one.
        assert_eq!(clean_value("  pad  \n".into(), "k").unwrap(), "  pad  ");
        assert_eq!(clean_value("a b\n".into(), "k").unwrap(), "a b");
    }

    #[test]
    fn test_clean_value_refuses_nothing_and_says_how() {
        for raw in ["", "\n", "\r\n"] {
            let err = clean_value(raw.into(), "db-password")
                .unwrap_err()
                .to_string();
            assert!(err.contains("db-password"), "{err}");
            assert!(err.contains("Pipe it in"), "the error must say how: {err}");
        }
    }
}
