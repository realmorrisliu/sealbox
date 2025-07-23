use crate::{ConfigCommands, config::Config, output::OutputManager};
use anyhow::{Context, Result};
use serde_json::json;

pub async fn handle_command(command: ConfigCommands, config: &mut Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        ConfigCommands::Show => show_config(config, &output).await,
        ConfigCommands::Set { key, value } => set_config(config, &key, &value, &output).await,
        ConfigCommands::Init => init_config(config, &output).await,
    }
}

async fn show_config(config: &Config, output: &OutputManager) -> Result<()> {
    let config_value = json!({
        "server": {
            "url": config.server.url,
            "token": if config.server.token.is_empty() { "<not set>" } else { "<configured>" }
        },
        "keys": {
            "public_key_path": config.keys.public_key_path,
            "private_key_path": config.keys.private_key_path
        },
        "output": {
            "format": config.output.format
        },
        "config_file": Config::config_file_path()?.display().to_string()
    });

    output.print_value(&config_value)?;
    Ok(())
}

async fn set_config(
    config: &mut Config,
    key: &str,
    value: &str,
    output: &OutputManager,
) -> Result<()> {
    match key {
        "server.url" => {
            config.server.url = value.to_string();
            output.print_success(&format!("Server URL set to: {value}"));
        }
        "server.token" => {
            config.server.token = value.to_string();
            output.print_success("Authentication token configured");
        }
        "keys.public_key_path" => {
            config.keys.public_key_path = value.into();
            output.print_success(&format!("Public key path set to: {value}"));
        }
        "keys.private_key_path" => {
            config.keys.private_key_path = value.into();
            output.print_success(&format!("Private key path set to: {value}"));
        }
        "output.format" => match value.to_lowercase().as_str() {
            "json" => {
                config.output.format = crate::config::OutputFormat::Json;
                output.print_success("Output format set to: JSON");
            }
            "yaml" => {
                config.output.format = crate::config::OutputFormat::Yaml;
                output.print_success("Output format set to: YAML");
            }
            "table" => {
                config.output.format = crate::config::OutputFormat::Table;
                output.print_success("Output format set to: Table");
            }
            _ => {
                anyhow::bail!(
                    "Invalid output format: {}. Supported formats: json, yaml, table",
                    value
                );
            }
        },
        _ => {
            anyhow::bail!(
                "Unknown configuration key: {}. Supported keys:\n  - server.url\n  - server.token\n  - keys.public_key_path\n  - keys.private_key_path\n  - output.format",
                key
            );
        }
    }

    config.save().context("Failed to save configuration")?;
    Ok(())
}

async fn init_config(config: &mut Config, output: &OutputManager) -> Result<()> {
    output.print_info("Starting configuration wizard...");

    // Ask for server URL
    println!("Enter server URL [{}]: ", config.server.url);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let server_url = input.trim();
    if !server_url.is_empty() {
        config.server.url = server_url.to_string();
    }

    // Ask for authentication token
    println!("Enter authentication token: ");
    let token = rpassword::read_password()?;
    if !token.is_empty() {
        config.server.token = token;
    }

    // Ask for key paths
    println!(
        "Enter public key file path [{}]: ",
        config.keys.public_key_path.display()
    );
    input.clear();
    std::io::stdin().read_line(&mut input)?;
    let public_key_path = input.trim();
    if !public_key_path.is_empty() {
        config.keys.public_key_path = public_key_path.into();
    }

    println!(
        "Enter private key file path [{}]: ",
        config.keys.private_key_path.display()
    );
    input.clear();
    std::io::stdin().read_line(&mut input)?;
    let private_key_path = input.trim();
    if !private_key_path.is_empty() {
        config.keys.private_key_path = private_key_path.into();
    }

    // Save configuration
    config.save().context("Failed to save configuration")?;
    output.print_success("Configuration initialization completed!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFormat;

    #[tokio::test]
    async fn test_show_config() {
        let config = Config::default();
        let output = OutputManager::new(OutputFormat::Json);

        // This test mainly verifies no panic occurs
        assert!(show_config(&config, &output).await.is_ok());
    }

    #[tokio::test]
    async fn test_set_config_server_url() {
        let mut config = Config::default();
        let output = OutputManager::new(OutputFormat::Json);

        // Due to need to save config file, only testing logic here
        let _result = set_config(&mut config, "server.url", "https://example.com", &output).await;

        // Configuration should be set, but may fail due to file system issues
        assert_eq!(config.server.url, "https://example.com");
    }

    #[test]
    fn test_invalid_config_key() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut config = Config::default();
            let output = OutputManager::new(OutputFormat::Json);

            let result = set_config(&mut config, "invalid.key", "value", &output).await;
            assert!(result.is_err());
        });
    }
}
