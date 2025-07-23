use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub keys: KeyConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyConfig {
    pub public_key_path: PathBuf,
    pub private_key_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OutputConfig {
    pub format: OutputFormat,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                url: "http://127.0.0.1:8080".to_string(),
                token: String::new(),
            },
            keys: KeyConfig {
                public_key_path: PathBuf::from("public_key.pem"),
                private_key_path: PathBuf::from("private_key.pem"),
            },
            output: OutputConfig {
                format: OutputFormat::Table,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let mut config: Config = toml::from_str(&config_content)
            .with_context(|| format!("Invalid config file format: {}", config_path.display()))?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let config_content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, config_content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        println!("Configuration saved to: {}", config_path.display());
        Ok(())
    }

    pub fn config_file_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Unable to determine home directory")?;

        Ok(home_dir.join(".sealbox").join("config.toml"))
    }

    #[allow(dead_code)]
    pub fn config_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Unable to determine home directory")?;

        Ok(home_dir.join(".sealbox"))
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(url) = std::env::var("SEALBOX_URL") {
            self.server.url = url;
        }

        if let Ok(token) = std::env::var("SEALBOX_TOKEN") {
            self.server.token = token;
        }

        if let Ok(public_key) = std::env::var("SEALBOX_PUBLIC_KEY") {
            self.keys.public_key_path = PathBuf::from(public_key);
        }

        if let Ok(private_key) = std::env::var("SEALBOX_PRIVATE_KEY") {
            self.keys.private_key_path = PathBuf::from(private_key);
        }

        if let Ok(format) = std::env::var("SEALBOX_OUTPUT_FORMAT") {
            match format.to_lowercase().as_str() {
                "json" => self.output.format = OutputFormat::Json,
                "yaml" => self.output.format = OutputFormat::Yaml,
                "table" => self.output.format = OutputFormat::Table,
                _ => {} // Keep default value
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.token.is_empty() {
            anyhow::bail!(
                "Server authentication token not configured. Please set SEALBOX_TOKEN environment variable or run 'sealbox config set token <your-token>'"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.url, "http://127.0.0.1:8080");
        assert_eq!(config.keys.public_key_path, PathBuf::from("public_key.pem"));
        assert_eq!(
            config.keys.private_key_path,
            PathBuf::from("private_key.pem")
        );
    }

    #[test]
    fn test_env_overrides() {
        unsafe {
            std::env::set_var("SEALBOX_URL", "https://example.com");
            std::env::set_var("SEALBOX_TOKEN", "test-token");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.server.url, "https://example.com");
        assert_eq!(config.server.token, "test-token");

        // Clean up environment variables
        unsafe {
            std::env::remove_var("SEALBOX_URL");
            std::env::remove_var("SEALBOX_TOKEN");
        }
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.server.token = "".to_string();

        assert!(config.validate().is_err());

        config.server.token = "valid-token".to_string();
        assert!(config.validate().is_ok());
    }
}
