use anyhow::{Context, Result};
use reqwest::Client;
use rsa::pkcs1::DecodeRsaPublicKey;
use serde_json::json;
use std::{fs, path::Path};
use uuid::Uuid;

use crate::{KeyCommands, config::Config, crypto::CryptoService, output::OutputManager};

pub async fn handle_command(command: KeyCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        KeyCommands::Generate {
            public_key_path,
            private_key_path,
            force,
        } => generate_keys(config, &output, public_key_path, private_key_path, force).await,
        KeyCommands::Register => register_key(config, &output).await,
        KeyCommands::List => list_keys(config, &output).await,
        KeyCommands::Rotate {
            new_key_id,
            old_key_id,
        } => rotate_keys(config, &output, new_key_id, old_key_id).await,
        KeyCommands::Status => check_key_status(config, &output).await,
    }
}

async fn generate_keys(
    config: &Config,
    output: &OutputManager,
    public_key_path: Option<String>,
    private_key_path: Option<String>,
    force: bool,
) -> Result<()> {
    let public_path = public_key_path
        .as_deref()
        .unwrap_or_else(|| config.keys.public_key_path.to_str().unwrap());
    let private_path = private_key_path
        .as_deref()
        .unwrap_or_else(|| config.keys.private_key_path.to_str().unwrap());

    // Check if files already exist
    if !force && (Path::new(public_path).exists() || Path::new(private_path).exists()) {
        anyhow::bail!(
            "Key files already exist:\n  Public key: {}\n  Private key: {}\n\nUse --force flag to overwrite existing files",
            public_path,
            private_path
        );
    }

    output.print_info("Generating RSA key pair...");

    let (private_key_pem, public_key_pem) = sealbox_server::crypto::master_key::generate_key_pair()
        .context("Failed to generate key pair")?;

    // Ensure directories exist
    if let Some(parent) = Path::new(private_path).parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create private key directory: {}",
                parent.display()
            )
        })?;
    }
    if let Some(parent) = Path::new(public_path).parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create public key directory: {}",
                parent.display()
            )
        })?;
    }

    // Save key files
    fs::write(private_path, private_key_pem)
        .with_context(|| format!("Failed to write private key file: {private_path}"))?;
    fs::write(public_path, public_key_pem)
        .with_context(|| format!("Failed to write public key file: {public_path}"))?;

    // Set private key file permissions (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(private_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(private_path, perms)?;
    }

    output.print_success("Key pair generated successfully!");
    output.print_info(&format!("Public key saved to: {public_path}"));
    output.print_info(&format!(
        "Private key saved to: {private_path} (permissions set to 600)"
    ));

    Ok(())
}

async fn register_key(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let public_key_path = config
        .keys
        .public_key_path
        .to_str()
        .context("Public key path contains invalid characters")?;

    if !Path::new(public_key_path).exists() {
        anyhow::bail!(
            "Public key file does not exist: {}. Please run 'sealbox key generate' first to generate key pair",
            public_key_path
        );
    }

    let public_key_pem = fs::read_to_string(public_key_path)
        .with_context(|| format!("Failed to read public key file: {public_key_path}"))?;

    // Validate public key format
    rsa::RsaPublicKey::from_pkcs1_pem(&public_key_pem)
        .with_context(|| format!("Invalid public key format: {public_key_path}"))?;

    output.print_info("Registering public key to server...");

    let client = Client::new();
    let response = client
        .post(format!("{}/v1/master-key", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&json!({ "public_key": public_key_pem }))
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if status.is_success() {
        let master_key: sealbox_server::repo::MasterKey = response
            .json()
            .await
            .context("Failed to parse server response")?;

        output.print_success("Public key registered successfully!");

        let formatted_keys = vec![master_key];
        output.print_master_keys(&formatted_keys)?;
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!("Server returned error (status code: {}):\n{}", status, error_body);
    }

    Ok(())
}

async fn list_keys(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    output.print_info("Fetching master key list...");

    let client = Client::new();
    let response = client
        .get(format!("{}/v1/master-key", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if status.is_success() {
        let master_keys: Vec<sealbox_server::repo::MasterKey> = response
            .json()
            .await
            .context("Failed to parse server response")?;

        if master_keys.is_empty() {
            output.print_info("No master keys on server");
        } else {
            output.print_master_keys(&master_keys)?;
        }
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!("Server returned error (status code: {}):\n{}", status, error_body);
    }

    Ok(())
}

async fn rotate_keys(
    config: &Config,
    output: &OutputManager,
    new_key_id: String,
    old_key_id: String,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let private_key_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path contains invalid characters")?;

    if !Path::new(private_key_path).exists() {
        anyhow::bail!(
            "Private key file does not exist: {}. Key rotation requires the old private key file",
            private_key_path
        );
    }

    let old_private_key_pem = fs::read_to_string(private_key_path)
        .with_context(|| format!("Failed to read private key file: {private_key_path}"))?;

    let new_key_uuid = Uuid::parse_str(&new_key_id)
        .with_context(|| format!("Invalid new key ID format: {new_key_id}"))?;
    let old_key_uuid = Uuid::parse_str(&old_key_id)
        .with_context(|| format!("Invalid old key ID format: {old_key_id}"))?;

    output.print_info("Performing key rotation...");
    output.print_warning("This operation will re-encrypt all secrets using the old key, please ensure the operation is correct!");

    let payload = json!({
        "new_master_key_id": new_key_uuid,
        "old_master_key_id": old_key_uuid,
        "old_private_key_pem": old_private_key_pem
    });

    let client = Client::new();
    let response = client
        .put(format!("{}/v1/master-key", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&payload)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if status.is_success() {
        let result: serde_json::Value = response.json().await.context("Failed to parse server response")?;

        output.print_success("Key rotation completed!");
        output.print_value(&result)?;

        if let Some(failed_keys) = result.get("failed_secret_keys") {
            if !failed_keys.as_array().unwrap_or(&vec![]).is_empty() {
                output.print_warning(
                    "The following secrets failed to rotate and may need manual handling:",
                );
                output.print_value(failed_keys)?;
            }
        }
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!("Server returned error (status code: {}):\n{}", status, error_body);
    }

    Ok(())
}

async fn check_key_status(config: &Config, output: &OutputManager) -> Result<()> {
    let public_key_path = config
        .keys
        .public_key_path
        .to_str()
        .context("Public key path contains invalid characters")?;
    let private_key_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path contains invalid characters")?;

    let mut status_info = json!({
        "local_keys": {
            "public_key_exists": Path::new(public_key_path).exists(),
            "private_key_exists": Path::new(private_key_path).exists(),
            "public_key_path": public_key_path,
            "private_key_path": private_key_path
        }
    });

    // Check if key pair matches
    if Path::new(public_key_path).exists() && Path::new(private_key_path).exists() {
        let mut crypto = CryptoService::new();

        match crypto.load_public_key(public_key_path) {
            Ok(()) => match crypto.load_private_key(private_key_path) {
                Ok(()) => match crypto.validate_key_pair() {
                    Ok(()) => {
                        status_info["local_keys"]["key_pair_valid"] = json!(true);
                    }
                    Err(e) => {
                        status_info["local_keys"]["key_pair_valid"] = json!(false);
                        status_info["local_keys"]["validation_error"] = json!(e.to_string());
                    }
                },
                Err(e) => {
                    status_info["local_keys"]["private_key_error"] = json!(e.to_string());
                }
            },
            Err(e) => {
                status_info["local_keys"]["public_key_error"] = json!(e.to_string());
            }
        }
    }

    // If configuration is valid, check keys on server
    if config.validate().is_ok() {
        match list_server_keys_internal(config).await {
            Ok(server_keys) => {
                status_info["server_keys"] = json!({
                    "count": server_keys.len(),
                    "keys": server_keys
                });
            }
            Err(e) => {
                status_info["server_keys"] = json!({
                    "error": e.to_string()
                });
            }
        }
    } else {
        status_info["server_keys"] = json!({
            "error": "Configuration is invalid, cannot check server keys"
        });
    }

    output.print_value(&status_info)?;
    Ok(())
}

async fn list_server_keys_internal(
    config: &Config,
) -> Result<Vec<sealbox_server::repo::MasterKey>> {
    let client = Client::new();
    let response = client
        .get(format!("{}/v1/master-key", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    if response.status().is_success() {
        response.json().await.context("Failed to parse server response")
    } else {
        anyhow::bail!("Server returned error: {}", response.status());
    }
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
        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_generate_keys() {
        let (config, _temp_dir) = create_test_config();
        let output = OutputManager::new(OutputFormat::Json);

        let result = generate_keys(&config, &output, None, None, true).await;
        assert!(result.is_ok());

        // Check if key files are generated
        assert!(config.keys.public_key_path.exists());
        assert!(config.keys.private_key_path.exists());
    }

    #[tokio::test]
    async fn test_generate_keys_without_force_existing_files() {
        let (config, _temp_dir) = create_test_config();
        let output = OutputManager::new(OutputFormat::Json);

        // Create empty file first
        fs::write(&config.keys.public_key_path, "").unwrap();

        let result = generate_keys(&config, &output, None, None, false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_key_status() {
        let (config, _temp_dir) = create_test_config();
        let output = OutputManager::new(OutputFormat::Json);

        // This test mainly verifies no panic occurs
        assert!(check_key_status(&config, &output).await.is_ok());
    }
}
