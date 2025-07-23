use std::{
    fs,
    path::{Path, PathBuf},
};

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
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".config").join("sealbox");

        Self {
            server: ServerConfig {
                url: "http://127.0.0.1:8080".to_string(),
                token: String::new(),
            },
            keys: KeyConfig {
                public_key_path: config_dir.join("public_key.pem"),
                private_key_path: config_dir.join("private_key.pem"),
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

        // Expand home directory paths
        config.expand_paths()?;

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

        Ok(home_dir.join(".config").join("sealbox").join("config.toml"))
    }

    #[allow(dead_code)]
    pub fn config_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Unable to determine home directory")?;

        Ok(home_dir.join(".config").join("sealbox"))
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

    fn expand_paths(&mut self) -> Result<()> {
        self.keys.public_key_path = Self::expand_home_dir(&self.keys.public_key_path)?;
        self.keys.private_key_path = Self::expand_home_dir(&self.keys.private_key_path)?;
        Ok(())
    }

    fn expand_home_dir(path: &Path) -> Result<PathBuf> {
        if let Some(path_str) = path.to_str() {
            if let Some(stripped) = path_str.strip_prefix("~/") {
                let home_dir = dirs::home_dir().context("Unable to determine home directory")?;
                return Ok(home_dir.join(stripped));
            }
        }
        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        // Should use ~/.config/sealbox for default paths
        assert!(
            config
                .keys
                .public_key_path
                .to_string_lossy()
                .contains(".config/sealbox")
        );
        assert!(
            config
                .keys
                .private_key_path
                .to_string_lossy()
                .contains(".config/sealbox")
        );
        assert_eq!(config.server.url, "http://127.0.0.1:8080");
        assert_eq!(config.server.token, "");
    }

    #[test]
    fn test_expand_home_dir() {
        let test_path = PathBuf::from("~/test/path");
        let expanded = Config::expand_home_dir(&test_path).unwrap();

        // Should expand ~ to home directory
        assert!(!expanded.to_string_lossy().starts_with("~"));
        assert!(expanded.to_string_lossy().ends_with("test/path"));
    }

    #[test]
    fn test_expand_paths() {
        let mut config = Config {
            server: ServerConfig {
                url: "http://test.com".to_string(),
                token: "test-token".to_string(),
            },
            keys: KeyConfig {
                public_key_path: PathBuf::from("~/test/public.pem"),
                private_key_path: PathBuf::from("~/test/private.pem"),
            },
            output: OutputConfig {
                format: OutputFormat::Json,
            },
        };

        config.expand_paths().unwrap();

        // Should expand ~ in all key paths
        assert!(
            !config
                .keys
                .public_key_path
                .to_string_lossy()
                .starts_with("~")
        );
        assert!(
            !config
                .keys
                .private_key_path
                .to_string_lossy()
                .starts_with("~")
        );
    }

    #[test]
    fn test_apply_env_overrides() {
        let mut config = Config::default();

        // Set environment variables
        unsafe {
            std::env::set_var("SEALBOX_URL", "http://env-test.com");
            std::env::set_var("SEALBOX_TOKEN", "env-token");
            std::env::set_var("SEALBOX_OUTPUT_FORMAT", "json");
        }

        config.apply_env_overrides();

        assert_eq!(config.server.url, "http://env-test.com");
        assert_eq!(config.server.token, "env-token");
        assert!(matches!(config.output.format, OutputFormat::Json));

        // Clean up
        unsafe {
            std::env::remove_var("SEALBOX_URL");
            std::env::remove_var("SEALBOX_TOKEN");
            std::env::remove_var("SEALBOX_OUTPUT_FORMAT");
        }
    }

    #[test]
    fn test_validate_empty_token() {
        let config = Config::default();

        // Should fail validation due to empty token
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_with_token() {
        let mut config = Config::default();
        config.server.token = "test-token".to_string();

        // Should pass validation
        assert!(config.validate().is_ok());
    }
}
