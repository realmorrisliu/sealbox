use std::env;
use tracing::{error, info};

/// Sealbox configuration struct
#[derive(Debug, Clone)]
pub struct SealboxConfig {
    /// Accepted only to create the very first admin identity, and only while none exists.
    /// Absent in normal operation; unset it once bootstrap is done.
    pub bootstrap_token: Option<String>,
    /// The server's public HTTPS URL. **This is the WebAuthn relying-party ID**, so every
    /// registered passkey is bound to it: changing the hostname invalidates all of them.
    /// Recovery does not depend on it — the recovery key decrypts the database independently of
    /// authentication — but it is not something to discover during an incident.
    pub public_url: String,
    /// How long after start-up the bootstrap token stays usable. Bounded because leaving the
    /// token in the environment after use is the normal outcome, not the exceptional one.
    pub bootstrap_window: std::time::Duration,
    pub store_path: String,
    pub listen_addr: String,
    /// Paths to the server's master keys (PEM), most-current first, comma-separated.
    ///
    /// The first becomes the key new secrets are encrypted under; any others are retired but
    /// still loaded, so their secrets remain readable and can be rekeyed onto the current key.
    /// A retired key is removed from this list once nothing references it (ADR 0001).
    pub master_key_paths: Vec<String>,
}

impl SealboxConfig {
    /// Load configuration from environment variables. Logs and returns Err if any required variable is missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        info!("Loading Sealbox configuration from environment variables...");

        // Optional: only a server with no identities has any use for it.
        let bootstrap_token = env::var("SEALBOX_BOOTSTRAP_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty());

        let public_url = match env::var("SEALBOX_PUBLIC_URL") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!(
                    "Environment variable SEALBOX_PUBLIC_URL is missing or empty. It is the \
                     WebAuthn relying-party ID that every admin passkey is bound to, so it \
                     cannot be guessed or defaulted."
                );
                return Err("SEALBOX_PUBLIC_URL is missing or empty".into());
            }
        };

        let bootstrap_window = env::var("SEALBOX_BOOTSTRAP_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(std::time::Duration::from_secs)
            .unwrap_or(std::time::Duration::from_secs(30 * 60));

        let store_path = match env::var("SEALBOX_STORE_PATH") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable SEALBOX_STORE_PATH is missing or empty");
                return Err("SEALBOX_STORE_PATH is missing or empty".into());
            }
        };

        let listen_addr = match env::var("SEALBOX_LISTEN_ADDR") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                error!("Environment variable SEALBOX_LISTEN_ADDR is missing or empty");
                return Err("SEALBOX_LISTEN_ADDR is missing or empty".into());
            }
        };

        let master_key_paths = match env::var("SEALBOX_MASTER_KEY_PATH") {
            Ok(val) if !val.trim().is_empty() => val
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>(),
            _ => {
                error!(
                    "Environment variable SEALBOX_MASTER_KEY_PATH is missing or empty. \
                     Sealbox cannot decrypt anything without its own master key; generate one \
                     and point this at it."
                );
                return Err("SEALBOX_MASTER_KEY_PATH is missing or empty".into());
            }
        };

        info!(
            "Sealbox configuration loaded: {:?}",
            SealboxConfig {
                bootstrap_token: bootstrap_token.as_ref().map(|_| "[HIDDEN]".to_string()),
                public_url: public_url.clone(),
                bootstrap_window,
                store_path: store_path.clone(),
                master_key_paths: master_key_paths.clone(),
                listen_addr: listen_addr.clone(),
            }
        );

        Ok(SealboxConfig {
            bootstrap_token,
            public_url,
            bootstrap_window,
            store_path,
            listen_addr,
            master_key_paths,
        })
    }
}

impl Default for SealboxConfig {
    fn default() -> Self {
        SealboxConfig {
            bootstrap_token: None,
            public_url: "http://localhost:8080".to_string(),
            bootstrap_window: std::time::Duration::from_secs(30 * 60),
            store_path: ":memory:".to_string(),
            listen_addr: "127.0.0.1:8080".to_string(),
            master_key_paths: Vec::new(),
        }
    }
}
