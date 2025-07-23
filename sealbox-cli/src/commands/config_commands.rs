use crate::{ConfigCommands, config::Config, output::OutputManager};
use anyhow::{Context, Result};
use serde_json::json;

struct InitOptions {
    url: Option<String>,
    token: Option<String>,
    public_key: Option<String>,
    private_key: Option<String>,
    output_format: Option<crate::OutputFormatArg>,
    force: bool,
}

pub async fn handle_command(command: ConfigCommands, config: &mut Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        ConfigCommands::Show => show_config(config, &output).await,
        ConfigCommands::Set { key, value } => set_config(config, &key, &value, &output).await,
        ConfigCommands::Init {
            url,
            token,
            public_key,
            private_key,
            output: output_format,
            force,
        } => {
            init_config(
                config,
                &output,
                InitOptions {
                    url,
                    token,
                    public_key,
                    private_key,
                    output_format,
                    force,
                },
            )
            .await
        }
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

async fn init_config(
    config: &mut Config,
    output: &OutputManager,
    options: InitOptions,
) -> Result<()> {
    // Check if config file exists and force flag
    let config_path = Config::config_file_path()?;
    if config_path.exists() && !options.force {
        anyhow::bail!(
            "Configuration file already exists: {}\nUse --force to overwrite",
            config_path.display()
        );
    }

    // Check which arguments need interactive input
    let needs_interactive = options.url.is_none()
        || options.token.is_none()
        || options.public_key.is_none()
        || options.private_key.is_none();

    // Apply command line arguments first
    if let Some(ref url) = options.url {
        config.server.url = url.clone();
    }
    if let Some(ref token) = options.token {
        config.server.token = token.clone();
    }
    if let Some(ref public_key) = options.public_key {
        config.keys.public_key_path = public_key.into();
    }
    if let Some(ref private_key) = options.private_key {
        config.keys.private_key_path = private_key.into();
    }
    if let Some(output_format) = options.output_format {
        config.output.format = output_format.into();
    }

    if needs_interactive {
        output.print_info("Starting configuration wizard...");

        // Ask for server URL only if not provided
        if options.url.is_none() {
            println!("Enter server URL [{}]: ", config.server.url);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let server_url = input.trim();
            if !server_url.is_empty() {
                config.server.url = server_url.to_string();
            }
        }

        // Ask for authentication token only if not provided
        if options.token.is_none() {
            println!("Enter authentication token: ");
            let token_input = rpassword::read_password()?;
            if !token_input.is_empty() {
                config.server.token = token_input;
            }
        }

        // Ask for key paths only if not provided
        if options.public_key.is_none() {
            println!(
                "Enter public key file path [{}]: ",
                config.keys.public_key_path.display()
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let public_key_path = input.trim();
            if !public_key_path.is_empty() {
                config.keys.public_key_path = public_key_path.into();
            }
        }

        if options.private_key.is_none() {
            println!(
                "Enter private key file path [{}]: ",
                config.keys.private_key_path.display()
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let private_key_path = input.trim();
            if !private_key_path.is_empty() {
                config.keys.private_key_path = private_key_path.into();
            }
        }
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
    use std::path::PathBuf;

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

    #[tokio::test]
    async fn test_init_config_with_args() {
        let mut config = Config::default();
        let output = OutputManager::new(OutputFormat::Json);

        // Test init with command line arguments
        let _result = init_config(
            &mut config,
            &output,
            InitOptions {
                url: Some("http://test.com".to_string()),
                token: Some("test-token".to_string()),
                public_key: Some("/tmp/public.pem".to_string()),
                private_key: Some("/tmp/private.pem".to_string()),
                output_format: Some(crate::OutputFormatArg::Json),
                force: true,
            },
        )
        .await;

        // Should set the values from arguments
        assert_eq!(config.server.url, "http://test.com");
        assert_eq!(config.server.token, "test-token");
        assert_eq!(
            config.keys.public_key_path,
            PathBuf::from("/tmp/public.pem")
        );
        assert_eq!(
            config.keys.private_key_path,
            PathBuf::from("/tmp/private.pem")
        );
        assert!(matches!(config.output.format, OutputFormat::Json));
    }

    #[test]
    fn test_set_config_all_keys() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = Config::default();
            let output = OutputManager::new(OutputFormat::Json);

            // Test all valid configuration keys
            let test_cases = vec![
                ("server.url", "https://example.com"),
                ("server.token", "test-token"),
                ("keys.public_key_path", "/path/to/public.pem"),
                ("keys.private_key_path", "/path/to/private.pem"),
                ("output.format", "json"),
            ];

            for (key, value) in test_cases {
                let mut test_config = config.clone();
                // Note: This will fail due to file system operations, but logic should work
                let _result = set_config(&mut test_config, key, value, &output).await;

                // Verify the values were set in memory
                match key {
                    "server.url" => assert_eq!(test_config.server.url, value),
                    "server.token" => assert_eq!(test_config.server.token, value),
                    "keys.public_key_path" => {
                        assert_eq!(test_config.keys.public_key_path, PathBuf::from(value))
                    }
                    "keys.private_key_path" => {
                        assert_eq!(test_config.keys.private_key_path, PathBuf::from(value))
                    }
                    "output.format" => {
                        assert!(matches!(test_config.output.format, OutputFormat::Json))
                    }
                    _ => {}
                }
            }
        });
    }
}
