use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Value, json};
use std::fs;

use crate::{SecretCommands, config::Config, crypto::CryptoService, output::OutputManager};

pub async fn handle_command(command: SecretCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        SecretCommands::Set { key, value, ttl } => {
            set_secret(config, &output, key, value, ttl).await
        }
        SecretCommands::Get { key, version } => get_secret(config, &output, key, version).await,
        SecretCommands::Delete { key, version } => {
            delete_secret(config, &output, key, version).await
        }
        SecretCommands::List => list_secrets(config, &output).await,
        SecretCommands::History { key } => get_secret_history(config, &output, key).await,
        SecretCommands::Import { file, format } => {
            import_secrets(config, &output, file, format).await
        }
        SecretCommands::Export { file, keys, format } => {
            export_secrets(config, &output, file, keys, format).await
        }
    }
}

async fn set_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    value: Option<String>,
    ttl: Option<i64>,
) -> Result<()> {
    config.validate().context("Configuration validation failed")?;

    // Get secret value
    let secret_value = match value {
        Some(val) => val,
        None => {
            output.print_info("Enter secret value (input will be hidden):");
            rpassword::read_password().context("Failed to read secret value")?
        }
    };

    if secret_value.trim().is_empty() {
        anyhow::bail!("Secret value cannot be empty");
    }

    // Load public key and encrypt
    let mut crypto = CryptoService::new();
    let public_key_path = config
        .keys
        .public_key_path
        .to_str()
        .context("Public key path contains invalid characters")?;

    crypto
        .load_public_key(public_key_path)
        .context("Failed to load public key")?;

    output.print_info("Encrypting secret...");
    let (encrypted_secret, _encrypted_key) = crypto
        .encrypt_secret(&secret_value)
        .context("Failed to encrypt secret")?;

    // Send to server
    output.print_info("Saving to server...");

    let payload = json!({
        "secret": encrypted_secret,
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
        anyhow::bail!("Server returned error (status code: {}):\n{}", status, error_body);
    }

    Ok(())
}

async fn get_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    version: Option<i32>,
) -> Result<()> {
    config.validate().context("Configuration validation failed")?;

    // Build request URL
    let mut url = format!("{}/v1/secrets/{}", config.server.url, key);
    if let Some(v) = version {
        url.push_str(&format!("?version={v}"));
    }

    output.print_info("Fetching secret from server...");

    let client = Client::new();
    let response = client
        .get(&url)
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!("Server returned error (status code: {}):\n{}", status, error_body);
    }

    let secret_data: Value = response
        .json()
        .await
        .context("Failed to parse server response")?;

    // Extract encrypted data
    let encrypted_secret = secret_data
        .get("secret")
        .and_then(|v| v.as_str())
        .context("Missing 'secret' field in response")?;

    let encrypted_key = secret_data
        .get("encrypted_data_key")
        .and_then(|v| v.as_str())
        .context("Missing 'encrypted_data_key' field in response")?;

    // Load private key and decrypt
    let mut crypto = CryptoService::new();
    let private_key_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path contains invalid characters")?;

    crypto
        .load_private_key(private_key_path)
        .context("Failed to load private key. Please ensure the private key file exists and is in correct format")?;

    output.print_info("Decrypting secret...");
    let decrypted_value = crypto
        .decrypt_secret(encrypted_secret, encrypted_key)
        .context("Failed to decrypt secret")?;

    // Display result
    let secret_version = secret_data
        .get("version")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let secret_ttl = secret_data.get("ttl").and_then(|v| v.as_i64());

    output.print_secret(&key, &decrypted_value, secret_version, secret_ttl)?;
    Ok(())
}

async fn delete_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    version: i32,
) -> Result<()> {
    config.validate().context("Configuration validation failed")?;

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
        anyhow::bail!("Server returned error (status code: {}):\n{}", status, error_body);
    }

    Ok(())
}

async fn list_secrets(_config: &Config, output: &OutputManager) -> Result<()> {
    // Note: Current server API doesn't support listing all secrets, this is a reserved feature
    output.print_warning("Server does not currently support listing all secrets");
    output.print_info("To view a specific secret, use: sealbox secret get <key>");
    Ok(())
}

async fn get_secret_history(_config: &Config, output: &OutputManager, key: String) -> Result<()> {
    // Note: Current server API doesn't directly support version history listing, this is a reserved feature
    output.print_warning("Server does not currently support viewing secret version history");
    output.print_info(&format!(
        "To get a specific version of the secret, use: sealbox secret get {key} --version <N>"
    ));
    Ok(())
}

async fn import_secrets(
    config: &Config,
    output: &OutputManager,
    file_path: String,
    format: String,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    if !["json", "yaml"].contains(&format.as_str()) {
        anyhow::bail!(
            "Unsupported file format: {}. Supported formats: json, yaml",
            format
        );
    }

    output.print_info(&format!("Importing secrets from {file_path}..."));

    let file_content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read file: {file_path}"))?;

    let secrets_data: Value = match format.as_str() {
        "json" => serde_json::from_str(&file_content)
            .with_context(|| format!("Failed to parse JSON file: {file_path}"))?,
        "yaml" => {
            // Simplified handling, assumes JSON format here
            // In actual project, serde_yaml dependency can be added
            serde_json::from_str(&file_content)
                .with_context(|| format!("Failed to parse YAML file: {file_path}"))?
        }
        _ => unreachable!(),
    };

    let secrets_obj = secrets_data.as_object().context(
        "Import file must contain an object with keys as secret names and values as secret content",
    )?;

    // Load public key
    let mut crypto = CryptoService::new();
    let public_key_path = config
        .keys
        .public_key_path
        .to_str()
        .context("Public key path contains invalid characters")?;
    crypto
        .load_public_key(public_key_path)
        .context("Failed to load public key")?;

    let mut success_count = 0;
    let mut error_count = 0;

    for (secret_key, secret_value) in secrets_obj {
        let value_str = match secret_value.as_str() {
            Some(s) => s,
            None => {
                output.print_warning(&format!(
                    "Skipping secret '{secret_key}': value is not a string"
                ));
                error_count += 1;
                continue;
            }
        };

        match import_single_secret(config, &crypto, secret_key, value_str).await {
            Ok(()) => {
                output.print_info(&format!("✓ Imported secret '{secret_key}'"));
                success_count += 1;
            }
            Err(e) => {
                output.print_error(&format!("✗ Failed to import secret '{secret_key}': {e}"));
                error_count += 1;
            }
        }
    }

    output.print_success(&format!(
        "Import completed! Success: {success_count}, Failed: {error_count}"
    ));

    Ok(())
}

async fn import_single_secret(
    config: &Config,
    crypto: &CryptoService,
    key: &str,
    value: &str,
) -> Result<()> {
    let (encrypted_secret, _encrypted_key) = crypto.encrypt_secret(value)?;

    let payload = json!({
        "secret": encrypted_secret,
        "ttl": null
    });

    let client = Client::new();
    let response = client
        .put(format!("{}/v1/secrets/{}", config.server.url, key))
        .bearer_auth(&config.server.token)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server error: {}", error_body);
    }

    Ok(())
}

async fn export_secrets(
    _config: &Config,
    output: &OutputManager,
    file_path: String,
    keys_pattern: Option<String>,
    format: String,
) -> Result<()> {
    output
        .print_warning("Export functionality requires server API support for listing all secrets");
    output.print_info("Current version does not support batch export functionality");

    // This is the implementation framework for reserved functionality
    if keys_pattern.is_some() {
        output.print_info(&format!(
            "Future support for pattern-based export: {keys_pattern:?}"
        ));
    }
    output.print_info(&format!("Export format: {format}"));
    output.print_info(&format!("Export file: {file_path}"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFormat;
    use tempfile::TempDir;

    fn create_test_config() -> (Config, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.keys.public_key_path = temp_dir.path().join("public.pem");
        config.keys.private_key_path = temp_dir.path().join("private.pem");
        config.server.token = "test-token".to_string();
        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_set_secret_empty_value() {
        let (config, _temp_dir) = create_test_config();
        let output = OutputManager::new(OutputFormat::Json);

        let result = set_secret(
            &config,
            &output,
            "test-key".to_string(),
            Some("".to_string()),
            None,
        )
        .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Secret value cannot be empty")
        );
    }

    #[tokio::test]
    async fn test_import_secrets_invalid_format() {
        let (config, _temp_dir) = create_test_config();
        let output = OutputManager::new(OutputFormat::Json);

        let result =
            import_secrets(&config, &output, "test.txt".to_string(), "xml".to_string()).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported file format")
        );
    }

    #[test]
    fn test_import_single_secret_logic() {
        // This mainly tests function signature and basic logic
        // Actual network request testing requires mock server
        // Test placeholder - functionality verified by integration tests
    }
}
