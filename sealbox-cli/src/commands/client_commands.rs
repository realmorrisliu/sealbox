use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{fs, path::Path};
use uuid::Uuid;

use crate::{ClientCommands, config::Config, output::OutputManager};

fn http() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("http client")
}

pub async fn handle_command(command: ClientCommands, config: &mut Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());
    match command {
        ClientCommands::Register { name, description } => {
            register(config, &output, name, description).await
        }
        ClientCommands::List => list(config, &output).await,
        ClientCommands::Disable { client_id } => disable(config, &output, client_id).await,
        ClientCommands::Rename {
            client_id,
            name,
            description,
        } => rename(config, &output, client_id, name, description).await,
        ClientCommands::Status => status(config, &output).await,
    }
}

pub async fn up(
    config: &mut Config,
    name: Option<String>,
    description: Option<String>,
    use_enroll: bool,
) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    // Ensure keys exist; if missing, generate a new pair
    let pub_path = config
        .keys
        .public_key_path
        .to_str()
        .context("Public key path invalid")?;
    let priv_path = config
        .keys
        .private_key_path
        .to_str()
        .context("Private key path invalid")?;

    if !Path::new(pub_path).exists() || !Path::new(priv_path).exists() {
        output.print_info("Generating key pair (not found)...");
        generate_and_write_keys(pub_path, priv_path)?;
        output.print_success("Key pair generated");
    }

    // Register client if not already
    if config.keys.client_id.is_none() {
        if use_enroll {
            output.print_info("Starting enrollment flow...");
            enroll_and_register(config, &output, name, description).await?;
        } else {
            output.print_info("Registering client...");
            register(config, &output, name, description).await?;
        }
    } else {
        output.print_info("Client already registered. Skipping registration.");
    }

    output.print_success("Setup complete");
    Ok(())
}

#[derive(Deserialize)]
struct EnrollBeginResp {
    code: String,
    verify_url: String,
    expires_at: i64,
}

#[derive(Deserialize)]
struct EnrollStatusResp {
    status: String,
    name: Option<String>,
}

async fn enroll_and_register(
    config: &mut Config,
    output: &OutputManager,
    name: Option<String>,
    description: Option<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    // Begin enrollment
    let client = http();
    let begin = client
        .post(format!("{}/v1/enroll", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server for enrollment")?;
    if !begin.status().is_success() {
        anyhow::bail!("Enrollment begin failed: {}", begin.status());
    }
    let body: EnrollBeginResp = begin.json().await.context("Invalid enroll response")?;
    output.print_info(&format!(
        "Enrollment code: {}\nVerify at: {}\nExpires at: {}",
        body.code, body.verify_url, body.expires_at
    ));

    // Poll for approval
    let start = time::OffsetDateTime::now_utc().unix_timestamp();
    let timeout = 600i64; // 10 minutes
    // Poll until approved; capture approved name if provided
    let approved_name: Option<String> = loop {
        let poll = client
            .get(format!("{}/v1/enroll/{}", config.server.url, body.code))
            .bearer_auth(&config.server.token)
            .send()
            .await
            .context("Failed to poll enrollment status")?;
        if !poll.status().is_success() {
            anyhow::bail!("Enrollment status error: {}", poll.status());
        }
        let st: EnrollStatusResp = poll.json().await.context("Invalid status response")?;
        match st.status.as_str() {
            "Approved" => {
                break st.name;
            }
            "Expired" => anyhow::bail!("Enrollment code expired"),
            _ => {
                // Pending
            }
        }
        if time::OffsetDateTime::now_utc().unix_timestamp() - start > timeout {
            anyhow::bail!("Enrollment polling timed out");
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    };

    let chosen_name = approved_name.or(name).unwrap_or_else(default_client_name);

    register(config, output, Some(chosen_name), description).await
}

async fn register(
    config: &mut Config,
    output: &OutputManager,
    name: Option<String>,
    description: Option<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let public_key_path = config
        .keys
        .public_key_path
        .to_str()
        .context("Public key path contains invalid characters")?;

    if !Path::new(public_key_path).exists() {
        output.print_info("Public key not found. Generating...");
        let private_key_path = config
            .keys
            .private_key_path
            .to_str()
            .context("Private key path contains invalid characters")?;
        generate_and_write_keys(public_key_path, private_key_path)?;
    }

    let public_key_pem = fs::read_to_string(public_key_path)
        .with_context(|| format!("Failed to read public key file: {public_key_path}"))?;

    let payload = json!({
        "name": name.unwrap_or_else(default_client_name),
        "public_key": public_key_pem,
        "description": description,
    });

    let client = http();
    let resp = client
        .post(format!("{}/v1/clients", config.server.url))
        .bearer_auth(&config.server.token)
        .json(&payload)
        .send()
        .await
        .context("Failed to request server")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Server error: {}: {}", status, body);
    }

    #[derive(Deserialize)]
    struct RegisterResponse {
        id: Uuid,
        name: Option<String>,
    }
    let body: RegisterResponse = resp.json().await.context("Invalid server response")?;
    config.keys.client_id = Some(body.id);
    config.save().context("Failed to save configuration")?;

    output.print_success(&format!(
        "Client registered: {} ({})",
        body.name.unwrap_or_else(|| body.id.to_string()),
        body.id
    ));
    Ok(())
}

async fn list(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;
    let client = http();
    let resp = client
        .get(format!("{}/v1/clients", config.server.url))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;
    if !resp.status().is_success() {
        anyhow::bail!("Server error: {}", resp.status());
    }
    let val: serde_json::Value = resp.json().await.context("Failed to parse response")?;
    output.print_value(&val)?;
    Ok(())
}

async fn disable(config: &Config, output: &OutputManager, client_id: String) -> Result<()> {
    let id = Uuid::parse_str(&client_id).context("Invalid client ID")?;
    let client = http();
    let resp = client
        .put(format!("{}/v1/clients/{}/status", config.server.url, id))
        .bearer_auth(&config.server.token)
        .json(&json!({ "status": "Disabled" }))
        .send()
        .await
        .context("Failed to request server")?;
    if !resp.status().is_success() {
        anyhow::bail!("Server error: {}", resp.status());
    }
    output.print_success(&format!("Client {id} disabled"));
    Ok(())
}

async fn rename(
    config: &Config,
    output: &OutputManager,
    client_id: String,
    name: String,
    description: Option<String>,
) -> Result<()> {
    let id = Uuid::parse_str(&client_id).context("Invalid client ID")?;
    let client = http();
    let resp = client
        .put(format!("{}/v1/clients/{}/name", config.server.url, id))
        .bearer_auth(&config.server.token)
        .json(&json!({ "name": name, "description": description }))
        .send()
        .await
        .context("Failed to request server")?;
    if !resp.status().is_success() {
        anyhow::bail!("Server error: {}", resp.status());
    }
    output.print_success(&format!("Client {id} updated"));
    Ok(())
}

async fn status(config: &Config, output: &OutputManager) -> Result<()> {
    let id = config.keys.client_id;
    let mut info = serde_json::json!({
        "client_id": id.map(|v| v.to_string()),
        "url": config.server.url,
        "public_key_path": config.keys.public_key_path,
        "private_key_path": config.keys.private_key_path,
    });
    if let Some(id) = id {
        // fetch associations count
        let client = http();
        if let Ok(resp) = client
            .get(format!("{}/v1/clients/{}/secrets", config.server.url, id))
            .bearer_auth(&config.server.token)
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    if let Some(arr) = v.get("associations").and_then(|x| x.as_array()) {
                        info["associations_count"] = json!(arr.len());
                    }
                }
            }
        }
    }
    output.print_value(&info)?;
    Ok(())
}

fn default_client_name() -> String {
    // Try common envs, fallback
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "sealbox-client".to_string())
}

fn generate_and_write_keys(public_path: &str, private_path: &str) -> Result<()> {
    let (private_pem, public_pem) = sealbox_server::crypto::client_key::generate_key_pair()
        .context("Failed to generate key pair")?;
    if let Some(parent) = Path::new(public_path).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create public key dir: {}", parent.display()))?;
    }
    if let Some(parent) = Path::new(private_path).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create private key dir: {}", parent.display()))?;
    }
    fs::write(private_path, private_pem)
        .with_context(|| format!("Failed to write private key: {private_path}"))?;
    fs::write(public_path, public_pem)
        .with_context(|| format!("Failed to write public key: {public_path}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(private_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(private_path, perms)?;
    }
    Ok(())
}
