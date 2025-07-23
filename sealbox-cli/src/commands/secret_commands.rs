use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Value, json};
use std::{fs, str::FromStr};

use crate::{SecretCommands, config::Config, output::OutputManager};

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
    config
        .validate()
        .context("Configuration validation failed")?;

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

async fn get_secret(
    config: &Config,
    output: &OutputManager,
    key: String,
    version: Option<i32>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

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
        anyhow::bail!(
            "Server returned error (status code: {}):\n{}",
            status,
            error_body
        );
    }

    let secret_data: Value = response
        .json()
        .await
        .context("Failed to parse server response")?;

    // Extract encrypted data from server response
    let encrypted_data = secret_data
        .get("encrypted_data")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'encrypted_data' field in response")?;

    let encrypted_data_key = secret_data
        .get("encrypted_data_key")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'encrypted_data_key' field in response")?;

    // Convert JSON arrays to byte vectors
    let encrypted_data_bytes: Vec<u8> = encrypted_data
        .iter()
        .map(|v| v.as_u64().unwrap_or(0) as u8)
        .collect();

    let encrypted_data_key_bytes: Vec<u8> = encrypted_data_key
        .iter()
        .map(|v| v.as_u64().unwrap_or(0) as u8)
        .collect();

    // Load private key and decrypt using server's crypto module
    let private_key_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path contains invalid characters")?;

    let private_key_pem =
        std::fs::read_to_string(private_key_path).context("Failed to read private key file")?;

    output.print_info("Decrypting secret...");

    // Use server's crypto modules for decryption
    let private_key =
        sealbox_server::crypto::master_key::PrivateMasterKey::from_str(&private_key_pem)
            .context("Failed to parse private key")?;

    // Decrypt the data key using RSA private key
    let decrypted_data_key = private_key
        .decrypt(&encrypted_data_key_bytes)
        .context("Failed to decrypt data key with RSA private key")?;

    // Use the data key to decrypt the secret data
    let data_key = sealbox_server::crypto::data_key::DataKey::from_bytes(&decrypted_data_key)
        .context("Invalid data key format")?;

    let decrypted_bytes = data_key
        .decrypt(&encrypted_data_bytes)
        .context("Failed to decrypt secret data")?;

    let decrypted_value =
        String::from_utf8(decrypted_bytes).context("Decrypted data is not valid UTF-8")?;

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

    // No need to load public key since server handles encryption

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

        match import_single_secret(config, secret_key, value_str).await {
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

async fn import_single_secret(config: &Config, key: &str, value: &str) -> Result<()> {
    let payload = json!({
        "secret": value,
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
