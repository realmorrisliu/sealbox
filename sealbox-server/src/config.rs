use std::{env, fs};
use tracing::{error, info};

/// Sealbox configuration struct
#[derive(Debug, Clone)]
pub struct SealboxConfig {
    pub auth_token: String,
    pub store_path: String,
    pub listen_addr: String,
    pub legacy_v1_enabled: bool,
}

impl SealboxConfig {
    /// Load configuration from environment variables. Logs and returns Err if any required variable is missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        info!("Loading Sealbox configuration from environment variables...");

        let auth_token = match Self::read_env_or_file("AUTH_TOKEN", "AUTH_TOKEN_FILE") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable AUTH_TOKEN or AUTH_TOKEN_FILE is missing or empty");
                return Err("AUTH_TOKEN or AUTH_TOKEN_FILE is missing or empty".into());
            }
        };

        let store_path = match env::var("STORE_PATH") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable STORE_PATH is missing or empty");
                return Err("STORE_PATH is missing or empty".into());
            }
        };

        let listen_addr = match env::var("LISTEN_ADDR") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable LISTEN_ADDR is missing or empty");
                return Err("LISTEN_ADDR is missing or empty".into());
            }
        };

        let legacy_v1_enabled = match env::var("LEGACY_V1_ENABLED") {
            Ok(value) => Self::parse_bool("LEGACY_V1_ENABLED", &value)?,
            Err(env::VarError::NotPresent) => true,
            Err(err) => return Err(format!("failed to read LEGACY_V1_ENABLED: {err}")),
        };

        info!(
            "Sealbox configuration loaded: {:?}",
            SealboxConfig {
                auth_token: "[HIDDEN]".to_string(),
                store_path: store_path.clone(),
                listen_addr: listen_addr.clone(),
                legacy_v1_enabled,
            }
        );

        Ok(SealboxConfig {
            auth_token,
            store_path,
            listen_addr,
            legacy_v1_enabled,
        })
    }

    fn read_env_or_file(value_var: &str, file_var: &str) -> Result<String, String> {
        if let Ok(path) = env::var(file_var) {
            let value = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read {file_var} path {path}: {err}"))?;
            return Ok(Self::trim_secret_file_value(value));
        }

        env::var(value_var).map_err(|err| err.to_string())
    }

    fn trim_secret_file_value(value: String) -> String {
        value.trim_end_matches(['\r', '\n']).to_string()
    }

    fn parse_bool(name: &str, value: &str) -> Result<bool, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(format!("{name} must be true or false")),
        }
    }
}

impl Default for SealboxConfig {
    fn default() -> Self {
        SealboxConfig {
            auth_token: "test-token".to_string(),
            store_path: ":memory:".to_string(),
            listen_addr: "127.0.0.1:8080".to_string(),
            legacy_v1_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_reads_auth_token_file() {
        let temp_dir = tempfile::tempdir().expect("Should create temp dir");
        let token_file = temp_dir.path().join("auth_token");
        fs::write(&token_file, "file-token\n").expect("Should write token file");

        unsafe {
            std::env::remove_var("AUTH_TOKEN");
            std::env::set_var("AUTH_TOKEN_FILE", &token_file);
            std::env::set_var("STORE_PATH", ":memory:");
            std::env::set_var("LISTEN_ADDR", "127.0.0.1:0");
            std::env::set_var("LEGACY_V1_ENABLED", "false");
        }

        let config = SealboxConfig::from_env().expect("Should load config");

        assert_eq!(config.auth_token, "file-token");
        assert!(!config.legacy_v1_enabled);

        unsafe {
            std::env::remove_var("AUTH_TOKEN_FILE");
            std::env::remove_var("STORE_PATH");
            std::env::remove_var("LISTEN_ADDR");
            std::env::remove_var("LEGACY_V1_ENABLED");
        }
    }

    #[test]
    fn rejects_invalid_legacy_v1_flag() {
        assert!(SealboxConfig::parse_bool("LEGACY_V1_ENABLED", "sometimes").is_err());
    }
}
