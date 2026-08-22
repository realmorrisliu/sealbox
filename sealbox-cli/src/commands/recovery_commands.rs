//! Backing up the one thing replication does not cover.
//!
//! Litestream copies the database. It does not copy the master key, and a database without its key
//! is ciphertext under something that no longer exists. This is how that stops being true.

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::{RecoveryCommands, config::Config, output::OutputManager};

pub async fn handle_command(command: RecoveryCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());
    match command {
        RecoveryCommands::Init { out, description } => {
            init(config, &output, out, description).await
        }
        RecoveryCommands::Export { id, out } => export(config, &output, id, out).await,
        RecoveryCommands::Restore { blob, key, out } => restore(&output, blob, key, out),
        RecoveryCommands::List => list(config, &output).await,
    }
}

/// Generate a recovery keypair, register the public half, and **prove the result works**.
async fn init(
    config: &Config,
    output: &OutputManager,
    out: String,
    description: Option<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    if std::path::Path::new(&out).exists() {
        anyhow::bail!(
            "{out} already exists. Refusing to overwrite it: if it is a recovery key still in \
             use, replacing it would leave a blob nothing can open."
        );
    }

    // Generated here and never sent. Only the public half leaves this machine, which is what makes
    // the stored blob safe to keep anywhere.
    let (private_pem, public_pem) = sealbox_server::crypto::master_key::generate_key_pair()
        .context("Failed to generate a recovery keypair")?;
    write_private(&out, &private_pem)?;

    let registered: serde_json::Value = send(
        config,
        Client::new()
            .post(format!("{}/v1/recovery", config.server.url))
            .json(&json!({ "public_key": public_pem, "description": description })),
    )
    .await?;

    let id = registered["recovery_key_id"]
        .as_str()
        .context("Server did not return a recovery key id")?;

    // The verification. ADR 0010 asked the operator to type the key back; a 1.7 KB PEM is not
    // transcribable, so this does the stronger thing directly — take the file just written, take
    // the blob the server stored, and recover the master key with them. An unverified backup is
    // reliably not a backup.
    let blob: serde_json::Value = send(
        config,
        Client::new().get(format!("{}/v1/recovery/{id}", config.server.url)),
    )
    .await?;
    let recovered = decrypt_blob(&blob, &private_pem)
        .context("The recovery key just written cannot open the blob the server stored")?;
    if !recovered.starts_with(b"-----BEGIN") {
        anyhow::bail!("The recovered material is not a key. Do not rely on this backup.");
    }

    output.print_success(&format!("Recovery verified. Key written to {out}."));
    output.print_info(&format!(
        "  recovery key id: {id}\n  master key: {}",
        registered["master_key_fingerprint"].as_str().unwrap_or("?")
    ));
    output.print_warning(
        "That file is the only thing that can recover this server's secrets. Move it into a \
         password manager or onto paper, then delete it from this machine — an agent on this \
         machine can read it exactly as easily as you can.",
    );
    Ok(())
}

async fn export(
    config: &Config,
    output: &OutputManager,
    id: String,
    out: Option<String>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let blob = send(
        config,
        Client::new().get(format!("{}/v1/recovery/{id}", config.server.url)),
    )
    .await?;

    match out {
        Some(path) => {
            std::fs::write(&path, serde_json::to_vec_pretty(&blob)?)
                .with_context(|| format!("Failed to write {path}"))?;
            output.print_success(&format!("Blob written to {path}."));
            output.print_info(
                "Safe to store anywhere: without the recovery private key it yields nothing.",
            );
        }
        None => output.print_value(&blob)?,
    }
    Ok(())
}

async fn list(config: &Config, output: &OutputManager) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;
    let result = send(
        config,
        Client::new().get(format!("{}/v1/recovery", config.server.url)),
    )
    .await?;
    output.print_value(&result)?;
    Ok(())
}

/// Blob plus key to `master.pem`. **No server involved** — recovery happens when the server is
/// gone, so a restore path that needs one is not a restore path.
fn restore(output: &OutputManager, blob: String, key: String, out: String) -> Result<()> {
    if std::path::Path::new(&out).exists() {
        anyhow::bail!("{out} already exists. Refusing to overwrite a master key.");
    }

    let blob: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&blob).with_context(|| format!("Failed to read {blob}"))?,
    )
    .context("That file is not a recovery blob")?;
    let private_pem =
        std::fs::read_to_string(&key).with_context(|| format!("Failed to read {key}"))?;

    let recovered = decrypt_blob(&blob, &private_pem).context(
        "That recovery key does not open this blob. They are a matched pair: a blob is encrypted \
         to exactly one recovery key.",
    )?;

    write_private(&out, &String::from_utf8_lossy(&recovered))?;
    output.print_success(&format!("Master key restored to {out}."));
    output.print_info(
        "Point SEALBOX_MASTER_KEY_PATH at it and start the server. Restore the database \
         separately — this is only the key.",
    );
    Ok(())
}

/// Open the envelope: the recovery private key opens the data key, the data key opens the payload.
fn decrypt_blob(blob: &serde_json::Value, private_pem: &str) -> Result<Vec<u8>> {
    use std::str::FromStr;

    let encrypted_data = bytes(blob, "encrypted_data")?;
    let encrypted_data_key = bytes(blob, "encrypted_data_key")?;

    let private = sealbox_server::crypto::master_key::PrivateMasterKey::from_str(private_pem)
        .context("That is not a usable private key")?;
    let data_key_bytes = private
        .decrypt(&encrypted_data_key)
        .context("Failed to open the data key")?;
    let data_key = sealbox_server::crypto::data_key::DataKey::from_bytes(&data_key_bytes)
        .context("Recovered data key is malformed")?;
    data_key
        .decrypt(&encrypted_data)
        .context("Failed to open the payload")
}

fn bytes(blob: &serde_json::Value, field: &str) -> Result<Vec<u8>> {
    blob.get(field)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_u64())
                .map(|v| v as u8)
                .collect()
        })
        .with_context(|| format!("Recovery blob is missing `{field}`"))
}

/// `0600` from creation rather than created and then tightened — a window in which a key is
/// world-readable is a window.
fn write_private(path: &str, pem: &str) -> Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("Failed to create {path}"))?;
    file.write_all(pem.as_bytes())
        .with_context(|| format!("Failed to write {path}"))?;
    Ok(())
}

async fn send(config: &Config, request: reqwest::RequestBuilder) -> Result<serde_json::Value> {
    let response = request
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("Server returned {status}: {body}");
    }
    Ok(serde_json::from_str(&body).unwrap_or(serde_json::Value::Null))
}
