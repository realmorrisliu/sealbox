use std::env;
use tracing::{error, info};

/// Sealbox configuration struct
#[derive(Debug, Clone)]
pub struct SealboxConfig {
    pub master_key: String,
    pub auth_token: String,
    pub store_path: String,
    pub listen_addr: String,
}

impl SealboxConfig {
    /// Load configuration from environment variables. Logs and returns Err if any required variable is missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        info!("Loading Sealbox configuration from environment variables...");

        let master_key = match env::var("MASTER_KEY") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable MASTER_KEY is missing or empty");
                return Err("MASTER_KEY is missing or empty".into());
            }
        };

        let auth_token = match env::var("AUTH_TOKEN") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable AUTH_TOKEN is missing or empty");
                return Err("AUTH_TOKEN is missing or empty".into());
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

        info!(
            "Sealbox configuration loaded: {:?}",
            SealboxConfig {
                master_key: "[HIDDEN]".to_string(),
                auth_token: "[HIDDEN]".to_string(),
                store_path: store_path.clone(),
                listen_addr: listen_addr.clone(),
            }
        );

        Ok(SealboxConfig {
            master_key,
            auth_token,
            store_path,
            listen_addr,
        })
    }
}
