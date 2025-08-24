use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Value, json};
use std::{fs, str::FromStr};

use crate::{SecretCommands, config::Config, output::OutputManager};

// Create configured HTTP client for API requests
fn create_http_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

// Format secrets as environment variables
fn format_as_env_vars(
    secrets: &std::collections::HashMap<String, String>,
    prefix: &Option<String>,
    with_export: bool,
) -> String {
    let mut lines = Vec::new();
    for (key, value) in secrets {
        let env_key = if let Some(p) = prefix {
            format!("{}_{}", p.to_uppercase(), key.to_uppercase())
        } else {
            key.to_uppercase()
        };
        
        if with_export {
            lines.push(format!("export {}=\"{}\"", env_key, value));
        } else {
            lines.push(format!("{}={}", env_key, value));
        }
    }
    lines.join("\n")
}

pub async fn handle_command(command: SecretCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        SecretCommands::Set { key, value, ttl } => {
            set_secret(config, &output, key, value, ttl).await
        }
        SecretCommands::Get { key, version } => get_secret(config, &output, key, version).await,
        SecretCommands::History { key } => get_secret_history(config, &output, key).await,
        SecretCommands::Import { file, format } => {
            import_secrets(config, &output, file, format).await
        }
        SecretCommands::Export { file, keys, format, prefix } => {
            export_secrets(config, &output, file, keys, format, prefix).await
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

    let client = create_http_client();
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

    let client = create_http_client();
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

    output.print_info("Decrypting secret...");

    let decrypted_value = decrypt_secret_response(config, &secret_data)?;

    // Display result
    let secret_version = secret_data
        .get("version")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let secret_ttl = secret_data.get("ttl").and_then(|v| v.as_i64());

    output.print_secret(&key, &decrypted_value, secret_version, secret_ttl)?;
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

    let client = create_http_client();
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
    config: &Config,
    output: &OutputManager,
    file_path: String,
    keys_pattern: Option<String>,
    format: String,
    prefix: Option<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    if !["json", "yaml", "env", "shell"].contains(&format.as_str()) {
        anyhow::bail!(
            "Unsupported export format: {}. Supported formats: json, yaml, env, shell",
            format
        );
    }

    output.print_info("Fetching secrets from server...");

    let client = create_http_client();
    let response = client
        .get(format!("{}/v1/secrets", config.server.url))
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

    let secrets_list: Value = response
        .json()
        .await
        .context("Failed to parse server response")?;

    let secrets_array = secrets_list
        .get("secrets")
        .and_then(|v| v.as_array())
        .context("Server response missing 'secrets' array")?;

    // Filter secrets by pattern if provided
    let filtered_secrets: Vec<_> = if let Some(pattern) = keys_pattern {
        secrets_array
            .iter()
            .filter(|secret| {
                if let Some(key) = secret.get("key").and_then(|v| v.as_str()) {
                    // Simple glob pattern matching (just * support for now)
                    if pattern.contains('*') {
                        simple_glob_match(&pattern, key)
                    } else {
                        key.contains(&pattern)
                    }
                } else {
                    false
                }
            })
            .collect()
    } else {
        secrets_array.iter().collect()
    };

    if filtered_secrets.is_empty() {
        output.print_warning("No secrets found matching the criteria");
        return Ok(());
    }

    // Fetch and decrypt each secret
    let mut decrypted_secrets = std::collections::HashMap::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for secret in filtered_secrets {
        if let Some(key) = secret.get("key").and_then(|v| v.as_str()) {
            output.print_info(&format!("Fetching secret '{}'...", key));
            
            match get_secret_value(config, key).await {
                Ok(decrypted_value) => {
                    decrypted_secrets.insert(key.to_string(), decrypted_value);
                    success_count += 1;
                }
                Err(e) => {
                    output.print_warning(&format!("Failed to fetch secret '{}': {}", key, e));
                    error_count += 1;
                }
            }
        }
    }

    if decrypted_secrets.is_empty() {
        output.print_error("No secrets could be successfully fetched and decrypted");
        return Ok(());
    }

    // Format the output
    let formatted_output = match format.as_str() {
        "json" => serde_json::to_string_pretty(&decrypted_secrets)
            .context("Failed to serialize secrets to JSON")?,
        "yaml" => {
            // For now, output as JSON since we don't have serde_yaml dependency
            // In production, this would use serde_yaml::to_string
            serde_json::to_string_pretty(&decrypted_secrets)
                .context("Failed to serialize secrets to YAML format")?
        }
        "env" => format_as_env_vars(&decrypted_secrets, &prefix, false),
        "shell" => format_as_env_vars(&decrypted_secrets, &prefix, true),
        _ => unreachable!(),
    };

    // Output to file or stdout
    if file_path == "-" {
        println!("{}", formatted_output);
    } else {
        fs::write(&file_path, formatted_output)
            .with_context(|| format!("Failed to write to file: {}", file_path))?;
        output.print_success(&format!("Exported {} secrets to {}", success_count, file_path));
    }

    if error_count > 0 {
        output.print_warning(&format!("{} secrets failed to export", error_count));
    }

    Ok(())
}

// Shared function to decrypt secret data from server response
fn decrypt_secret_response(config: &Config, secret_data: &Value) -> Result<String> {
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

    // Load private key
    let private_key_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path contains invalid characters")?;

    let private_key_pem = fs::read_to_string(private_key_path)
        .context("Failed to read private key file")?;

    let private_key = sealbox_server::crypto::client_key::PrivateClientKey::from_str(&private_key_pem)
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

    String::from_utf8(decrypted_bytes).context("Decrypted data is not valid UTF-8")
}

// Helper function to fetch and decrypt a single secret
async fn get_secret_value(config: &Config, key: &str) -> Result<String> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/v1/secrets/{}", config.server.url, key))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch secret: HTTP {}", response.status());
    }

    let secret_data: Value = response
        .json()
        .await
        .context("Failed to parse server response")?;

    decrypt_secret_response(config, &secret_data)
}

// Simple glob pattern matching function (supports common patterns)
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    
    if pattern == "*" {
        return true;
    }
    
    if pattern.starts_with('*') && pattern.ends_with('*') {
        // Pattern like "*middle*" - check if text contains the middle part
        let middle = &pattern[1..pattern.len()-1];
        return middle.is_empty() || text.contains(middle);
    }
    
    if pattern.starts_with('*') {
        // Pattern like "*suffix" - check if text ends with suffix
        let suffix = &pattern[1..];
        return text.ends_with(suffix);
    }
    
    if pattern.ends_with('*') {
        // Pattern like "prefix*" - check if text starts with prefix
        let prefix = &pattern[..pattern.len()-1];
        return text.starts_with(prefix);
    }
    
    // For more complex patterns, fall back to simple substring search
    // This handles patterns like "a*b*c" by checking if all parts exist in order
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        
        if i == 0 {
            // First part must be at the beginning
            if !text[pos..].starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if let Some(found_pos) = text[pos..].find(part) {
            pos += found_pos + part.len();
        } else {
            return false;
        }
    }
    
    true
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
    fn test_format_as_env_vars_without_prefix() {
        let mut secrets = std::collections::HashMap::new();
        secrets.insert("database_url".to_string(), "postgres://localhost".to_string());
        secrets.insert("api_key".to_string(), "secret123".to_string());

        let result = format_as_env_vars(&secrets, &None, false);
        
        assert!(result.contains("DATABASE_URL=postgres://localhost"));
        assert!(result.contains("API_KEY=secret123"));
        assert!(!result.contains("export"));
    }

    #[test]
    fn test_format_as_env_vars_with_prefix() {
        let mut secrets = std::collections::HashMap::new();
        secrets.insert("db_host".to_string(), "localhost".to_string());

        let result = format_as_env_vars(&secrets, &Some("MY_APP".to_string()), false);
        
        assert_eq!(result, "MY_APP_DB_HOST=localhost");
    }

    #[test]
    fn test_format_as_env_vars_with_export() {
        let mut secrets = std::collections::HashMap::new();
        secrets.insert("port".to_string(), "8080".to_string());

        let result = format_as_env_vars(&secrets, &None, true);
        
        assert_eq!(result, "export PORT=\"8080\"");
    }

    #[test]
    fn test_simple_glob_match_exact() {
        assert!(simple_glob_match("hello", "hello"));
        assert!(!simple_glob_match("hello", "world"));
    }

    #[test]
    fn test_simple_glob_match_wildcard() {
        assert!(simple_glob_match("*", "anything"));
        assert!(simple_glob_match("*", ""));
    }

    #[test]
    fn test_simple_glob_match_prefix() {
        assert!(simple_glob_match("db_*", "db_host"));
        assert!(simple_glob_match("db_*", "db_"));
        assert!(!simple_glob_match("db_*", "api_key"));
    }

    #[test]
    fn test_simple_glob_match_suffix() {
        assert!(simple_glob_match("*_config", "database_config"));
        assert!(simple_glob_match("*_config", "_config"));
        assert!(!simple_glob_match("*_config", "database_secret"));
    }

    #[test]
    fn test_simple_glob_match_middle() {
        assert!(simple_glob_match("*test*", "my_test_data"));
        assert!(simple_glob_match("*test*", "test"));
        assert!(simple_glob_match("*test*", "testdata"));
        assert!(!simple_glob_match("*test*", "my_data"));
    }

    #[test]
    fn test_import_single_secret_logic() {
        // This mainly tests function signature and basic logic
        // Actual network request testing requires mock server
        // Test placeholder - functionality verified by integration tests
    }

}
