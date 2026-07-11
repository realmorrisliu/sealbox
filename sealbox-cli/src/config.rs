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
    #[serde(default = "default_api_version")]
    pub api_version: String,
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
                api_version: default_api_version(),
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

        let mut config = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;

            toml::from_str(&config_content)
                .with_context(|| format!("Invalid config file format: {}", config_path.display()))?
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides()?;

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

    fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(url) = std::env::var("SEALBOX_URL") {
            self.server.url = url;
        }

        if let Ok(token) = std::env::var("SEALBOX_TOKEN") {
            self.server.token = token;
        }

        if let Ok(token_file) = std::env::var("SEALBOX_TOKEN_FILE") {
            self.server.token = Self::read_secret_file("SEALBOX_TOKEN_FILE", &token_file)?;
        }

        if let Ok(api_version) = std::env::var("SEALBOX_API_VERSION") {
            self.server.api_version = normalize_api_version(&api_version)?;
        }

        if let Ok(public_key) = std::env::var("SEALBOX_PUBLIC_KEY") {
            self.keys.public_key_path = PathBuf::from(public_key);
        }

        if let Ok(public_key_file) = std::env::var("SEALBOX_PUBLIC_KEY_FILE") {
            self.keys.public_key_path = PathBuf::from(public_key_file);
        }

        if let Ok(private_key) = std::env::var("SEALBOX_PRIVATE_KEY") {
            self.keys.private_key_path = PathBuf::from(private_key);
        }

        if let Ok(private_key_file) = std::env::var("SEALBOX_PRIVATE_KEY_FILE") {
            self.keys.private_key_path = PathBuf::from(private_key_file);
        }

        if let Ok(format) = std::env::var("SEALBOX_OUTPUT_FORMAT") {
            match format.to_lowercase().as_str() {
                "json" => self.output.format = OutputFormat::Json,
                "yaml" => self.output.format = OutputFormat::Yaml,
                "table" => self.output.format = OutputFormat::Table,
                _ => {} // Keep default value
            }
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.token.is_empty() {
            anyhow::bail!(
                "Server authentication token not configured. Please set SEALBOX_TOKEN environment variable or run 'sealbox config set token <your-token>'"
            );
        }
        normalize_api_version(&self.server.api_version)?;

        Ok(())
    }

    pub fn api_url(&self, path: &str) -> String {
        format!(
            "{}/{}/{}",
            self.server.url.trim_end_matches('/'),
            self.server.api_version,
            path.trim_start_matches('/')
        )
    }

    pub fn admin_url(&self, path: &str) -> String {
        format!(
            "{}/v2/admin/{}",
            self.server.url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn expand_paths(&mut self) -> Result<()> {
        self.keys.public_key_path = Self::expand_home_dir(&self.keys.public_key_path)?;
        self.keys.private_key_path = Self::expand_home_dir(&self.keys.private_key_path)?;
        Ok(())
    }

    fn expand_home_dir(path: &Path) -> Result<PathBuf> {
        if let Some(path_str) = path.to_str()
            && let Some(stripped) = path_str.strip_prefix("~/")
        {
            let home_dir = dirs::home_dir().context("Unable to determine home directory")?;
            return Ok(home_dir.join(stripped));
        }
        Ok(path.to_path_buf())
    }

    fn read_secret_file(var_name: &str, path: &str) -> Result<String> {
        let value = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {var_name}: {path}"))?;
        Ok(value.trim_end_matches(['\r', '\n']).to_string())
    }
}

fn default_api_version() -> String {
    "v1".to_string()
}

pub(crate) fn normalize_api_version(value: &str) -> Result<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "v1" | "1" => Ok("v1".to_string()),
        "v2" | "2" => Ok("v2".to_string()),
        other => anyhow::bail!("Unsupported Sealbox API version: {other}. Expected v1 or v2."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
                api_version: "v1".to_string(),
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
        let _guard = ENV_LOCK.lock().unwrap();
        let mut config = Config::default();

        // Set environment variables
        unsafe {
            std::env::set_var("SEALBOX_URL", "http://env-test.com");
            std::env::set_var("SEALBOX_TOKEN", "env-token");
            std::env::set_var("SEALBOX_OUTPUT_FORMAT", "json");
        }

        config.apply_env_overrides().unwrap();

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
    fn test_apply_env_file_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let token_file = temp_dir.path().join("token");
        let public_key_file = temp_dir.path().join("public.pem");
        let private_key_file = temp_dir.path().join("private.pem");

        fs::write(&token_file, "file-token\n").unwrap();
        fs::write(&public_key_file, "public-key").unwrap();
        fs::write(&private_key_file, "private-key").unwrap();

        let mut config = Config::default();

        unsafe {
            std::env::set_var("SEALBOX_TOKEN_FILE", &token_file);
            std::env::set_var("SEALBOX_PUBLIC_KEY_FILE", &public_key_file);
            std::env::set_var("SEALBOX_PRIVATE_KEY_FILE", &private_key_file);
        }

        config.apply_env_overrides().unwrap();

        assert_eq!(config.server.token, "file-token");
        assert_eq!(config.server.api_version, "v1");
        assert_eq!(config.keys.public_key_path, public_key_file);
        assert_eq!(config.keys.private_key_path, private_key_file);

        unsafe {
            std::env::remove_var("SEALBOX_TOKEN_FILE");
            std::env::remove_var("SEALBOX_PUBLIC_KEY_FILE");
            std::env::remove_var("SEALBOX_PRIVATE_KEY_FILE");
        }
    }

    #[test]
    fn test_load_applies_env_overrides_without_config_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let token_file = temp_dir.path().join("token");
        let public_key_file = temp_dir.path().join("public.pem");
        let private_key_file = temp_dir.path().join("private.pem");
        fs::write(&token_file, "file-token\n").unwrap();
        fs::write(&public_key_file, "public-key").unwrap();
        fs::write(&private_key_file, "private-key").unwrap();

        let original_home = std::env::var_os("HOME");

        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("SEALBOX_URL", "http://env-only.test");
            std::env::set_var("SEALBOX_TOKEN_FILE", &token_file);
            std::env::set_var("SEALBOX_PUBLIC_KEY_FILE", &public_key_file);
            std::env::set_var("SEALBOX_PRIVATE_KEY_FILE", &private_key_file);
            std::env::set_var("SEALBOX_OUTPUT_FORMAT", "json");
        }

        let config = Config::load().unwrap();

        assert_eq!(config.server.url, "http://env-only.test");
        assert_eq!(config.server.token, "file-token");
        assert_eq!(config.keys.public_key_path, public_key_file);
        assert_eq!(config.keys.private_key_path, private_key_file);
        assert!(matches!(config.output.format, OutputFormat::Json));

        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
            std::env::remove_var("SEALBOX_URL");
            std::env::remove_var("SEALBOX_TOKEN_FILE");
            std::env::remove_var("SEALBOX_PUBLIC_KEY_FILE");
            std::env::remove_var("SEALBOX_PRIVATE_KEY_FILE");
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
