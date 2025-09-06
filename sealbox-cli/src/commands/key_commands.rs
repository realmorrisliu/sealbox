use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use rsa::pkcs1::DecodeRsaPublicKey;
use serde_json::json;
use std::{fs, path::Path};
use uuid::Uuid;

use crate::{KeyCommands, config::Config, output::OutputManager};

// Create configured HTTP client for API requests
fn create_http_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

pub async fn handle_command(command: KeyCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        KeyCommands::Generate {
            public_key_path,
            private_key_path,
            force,
        } => generate_keys(config, &output, public_key_path, private_key_path, force).await,
        KeyCommands::Register => register_key(config, &output).await,
        KeyCommands::Rotate {} => rotate_keys(config, &output).await,
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

    let (private_key_pem, public_key_pem) = sealbox_server::crypto::client_key::generate_key_pair()
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

    let client = create_http_client();
    let response = client
        .post(format!("{}/v1/client-key", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&json!({ "public_key": public_key_pem }))
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if status.is_success() {
        let client_key: sealbox_server::repo::ClientKey = response
            .json()
            .await
            .context("Failed to parse server response")?;

        output.print_success("Public key registered successfully!");

        let formatted_keys = vec![client_key];
        output.print_client_keys(&formatted_keys)?;

        // Persist client_id into config for future rotation
        let mut new_config = config.clone();
        new_config.keys.client_id = Some(formatted_keys[0].id);
        new_config
            .save()
            .context("Failed to save configuration with client_id")?;
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

async fn rotate_keys(config: &Config, output: &OutputManager) -> Result<()> {
    use std::str::FromStr;

    config
        .validate()
        .context("Configuration validation failed")?;

    // Determine client_id from server by matching registration or require explicit config
    // For now, require that the client has been registered previously and its id is stored server-side.
    // We derive the client_id by ensuring exactly one matching client exists on the server for our key; however
    // server list does not return public keys. We therefore rely on the id captured at registration time.

    // Load old private key
    let private_key_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path contains invalid characters")?;
    if !Path::new(private_key_path).exists() {
        anyhow::bail!(
            "Private key file does not exist: {}. Run 'sealbox key generate' and 'sealbox key register' first.",
            private_key_path
        );
    }
    let old_private_key_pem = fs::read_to_string(private_key_path)
        .with_context(|| format!("Failed to read private key file: {private_key_path}"))?;
    let old_private_key =
        sealbox_server::crypto::client_key::PrivateClientKey::from_str(&old_private_key_pem)
            .context("Failed to parse private key")?;

    // Determine client_id via server /v1/client-key list -> pick newest? Better: require single key returned and use its id
    let client_id = resolve_current_client_id(config)
        .await
        .context("Failed to resolve current client id. Please ensure you have registered your key using 'sealbox key register'.")?;

    // 1) Generate a new key pair locally (do not overwrite files yet)
    let (new_private_pem, new_public_pem) = sealbox_server::crypto::client_key::generate_key_pair()
        .context("Failed to generate new key pair")?;
    let new_public_key =
        sealbox_server::crypto::client_key::PublicClientKey::from_str(&new_public_pem)
            .context("Failed to parse generated public key")?;

    // 2) List all associations for this client
    let associations = list_client_associations(config, &client_id).await?;
    if associations.is_empty() {
        output.print_warning("No secret associations found for this client. Proceeding to update server public key only.");
    }

    // 3) For each association: decrypt DataKey with old private key, re-encrypt with new public key, upload
    let client = create_http_client();
    let mut failures: Vec<(String, i32, String)> = Vec::new();
    let mut updated = 0usize;
    for a in &associations {
        let data_key_bytes = match old_private_key.decrypt(&a.encrypted_data_key) {
            Ok(b) => b,
            Err(e) => {
                failures.push((
                    a.secret_key.clone(),
                    a.secret_version,
                    format!("decrypt failed: {e}"),
                ));
                continue;
            }
        };
        let new_edk = match new_public_key.encrypt(&data_key_bytes) {
            Ok(b) => b,
            Err(e) => {
                failures.push((
                    a.secret_key.clone(),
                    a.secret_version,
                    format!("encrypt failed: {e}"),
                ));
                continue;
            }
        };

        let resp = client
            .put(format!(
                "{}/v1/secrets/{}/permissions/{}/data-key",
                config.server.url, a.secret_key, client_id
            ))
            .bearer_auth(&config.server.token)
            .json(&json!({
                "secret_version": a.secret_version,
                "new_encrypted_data_key": new_edk,
            }))
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to update data key for secret '{}' version {}",
                    a.secret_key, a.secret_version
                )
            })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            failures.push((
                a.secret_key.clone(),
                a.secret_version,
                format!("server error: {status}: {body}"),
            ));
        } else {
            updated += 1;
        }
    }

    if !failures.is_empty() {
        output.print_error("Some associations failed to update. Aborting rotation before updating server public key or local files.");
        output.print_value(&json!({ "failures": failures }))?;
        return Err(anyhow!("{} association(s) failed", failures.len()));
    }

    // 4) Update server public key
    let resp = client
        .put(format!(
            "{}/v1/clients/{}/public-key",
            config.server.url, client_id
        ))
        .bearer_auth(&config.server.token)
        .json(&json!({ "new_public_key": new_public_pem }))
        .send()
        .await
        .context("Failed to update server public key")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Server public key update failed: {}: {}",
            status,
            body
        ));
    }

    // 5) Persist new local keys (with backup)
    backup_and_write_new_keys(config, &new_private_pem, &new_public_pem, output)?;

    output.print_success(&format!(
        "Key rotation completed successfully. Updated {updated} association(s) and server public key."
    ));
    Ok(())
}

#[derive(serde::Deserialize)]
struct ClientAssociationItem {
    secret_key: String,
    secret_version: i32,
    encrypted_data_key: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct ClientAssociationsResponse {
    associations: Vec<ClientAssociationItem>,
}

async fn list_client_associations(
    config: &Config,
    client_id: &Uuid,
) -> Result<Vec<ClientAssociationItem>> {
    let client = create_http_client();
    let res = client
        .get(format!(
            "{}/v1/clients/{}/secrets",
            config.server.url, client_id
        ))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to list client associations")?;
    if !res.status().is_success() {
        anyhow::bail!("Server returned error: {}", res.status());
    }
    let body: ClientAssociationsResponse = res
        .json()
        .await
        .context("Failed to parse associations response")?;
    Ok(body.associations)
}

async fn resolve_current_client_id(config: &Config) -> Result<Uuid> {
    if let Some(id) = config.keys.client_id {
        return Ok(id);
    }
    // We attempt to resolve by listing current client keys and picking the earliest Active one as a fallback.
    // But server hides public_key in list; we cannot match reliably. For safety, we use the single-key assumption.
    let keys = list_server_keys_internal(config).await?;
    if keys.is_empty() {
        return Err(anyhow!(
            "No client keys found on server. Run 'sealbox key register' first."
        ));
    }
    if keys.len() > 1 {
        // pick the most recently used if available
        let mut keys_sorted = keys;
        keys_sorted.sort_by_key(|k| k.last_used_at.unwrap_or_default());
        let picked = keys_sorted.last().unwrap();
        Ok(picked.id)
    } else {
        Ok(keys[0].id)
    }
}

fn backup_and_write_new_keys(
    config: &Config,
    new_private_pem: &str,
    new_public_pem: &str,
    output: &OutputManager,
) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let public_path = config.keys.public_key_path.to_str().unwrap();
    let private_path = config.keys.private_key_path.to_str().unwrap();
    let public_backup = format!("{public_path}.bak.{ts}");
    let private_backup = format!("{private_path}.bak.{ts}");

    // Backup existing files if present
    if Path::new(public_path).exists() {
        fs::copy(public_path, &public_backup)
            .with_context(|| format!("Failed to backup public key to {public_backup}"))?;
        output.print_info(&format!("Backed up public key to {public_backup}"));
    }
    if Path::new(private_path).exists() {
        fs::copy(private_path, &private_backup)
            .with_context(|| format!("Failed to backup private key to {private_backup}"))?;
        output.print_info(&format!("Backed up private key to {private_backup}"));
    }

    // Write new keys
    fs::write(private_path, new_private_pem)
        .with_context(|| format!("Failed to write private key file: {private_path}"))?;
    fs::write(public_path, new_public_pem)
        .with_context(|| format!("Failed to write public key file: {public_path}"))?;

    // Set private key file permissions (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(private_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(private_path, perms)?;
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

    // Check if key pair matches by reading and parsing both key files
    if Path::new(public_key_path).exists() && Path::new(private_key_path).exists() {
        use std::str::FromStr;

        match fs::read_to_string(public_key_path) {
            Ok(public_pem) => match fs::read_to_string(private_key_path) {
                Ok(private_pem) => {
                    // Try to parse both keys
                    match (
                        sealbox_server::crypto::client_key::PublicClientKey::from_str(&public_pem),
                        sealbox_server::crypto::client_key::PrivateClientKey::from_str(
                            &private_pem,
                        ),
                    ) {
                        (Ok(public_key), Ok(private_key)) => {
                            // Test key pair compatibility by encrypting and decrypting a test message
                            match public_key.encrypt(b"test") {
                                Ok(encrypted) => match private_key.decrypt(&encrypted) {
                                    Ok(decrypted) if decrypted == b"test" => {
                                        status_info["local_keys"]["key_pair_valid"] = json!(true);
                                    }
                                    Ok(_) => {
                                        status_info["local_keys"]["key_pair_valid"] = json!(false);
                                        status_info["local_keys"]["validation_error"] = json!(
                                            "Key pair mismatch: decryption result doesn't match"
                                        );
                                    }
                                    Err(e) => {
                                        status_info["local_keys"]["key_pair_valid"] = json!(false);
                                        status_info["local_keys"]["validation_error"] =
                                            json!(format!("Decryption failed: {}", e));
                                    }
                                },
                                Err(e) => {
                                    status_info["local_keys"]["key_pair_valid"] = json!(false);
                                    status_info["local_keys"]["validation_error"] =
                                        json!(format!("Encryption failed: {}", e));
                                }
                            }
                        }
                        (Err(e), _) => {
                            status_info["local_keys"]["public_key_error"] =
                                json!(format!("Failed to parse public key: {}", e));
                        }
                        (_, Err(e)) => {
                            status_info["local_keys"]["private_key_error"] =
                                json!(format!("Failed to parse private key: {}", e));
                        }
                    }
                }
                Err(e) => {
                    status_info["local_keys"]["private_key_error"] =
                        json!(format!("Failed to read private key file: {}", e));
                }
            },
            Err(e) => {
                status_info["local_keys"]["public_key_error"] =
                    json!(format!("Failed to read public key file: {}", e));
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
) -> Result<Vec<sealbox_server::repo::ClientKey>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/v1/client-key", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    if response.status().is_success() {
        response
            .json()
            .await
            .context("Failed to parse server response")
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
