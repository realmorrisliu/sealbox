use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use reqwest::{Client, Method};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{TenantCommands, TenantTokenCommands, config::Config, output::OutputManager};

pub async fn handle_command(command: TenantCommands, config: &Config) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;
    let output = OutputManager::new(config.output.format.clone());
    match command {
        TenantCommands::Create {
            display_name,
            token_label,
            token_expires_at,
            token_file,
        } => {
            let payload = json!({
                "display_name": display_name,
                "token_label": token_label,
                "token_expires_at": token_expires_at,
            });
            let response = request_json(
                config,
                Method::POST,
                config.admin_url("tenants"),
                Some(payload),
            )
            .await?;
            persist_issued_token(response, Path::new(&token_file), &output)
        }
        TenantCommands::List => {
            let response =
                request_json(config, Method::GET, config.admin_url("tenants"), None).await?;
            output.print_value(&response)
        }
        TenantCommands::Get { tenant_id } => {
            let response = request_json(
                config,
                Method::GET,
                config.admin_url(&format!("tenants/{tenant_id}")),
                None,
            )
            .await?;
            output.print_value(&response)
        }
        TenantCommands::Suspend { tenant_id } => {
            let response = request_json(
                config,
                Method::POST,
                config.admin_url(&format!("tenants/{tenant_id}/suspend")),
                None,
            )
            .await?;
            output.print_value(&response)
        }
        TenantCommands::Resume { tenant_id } => {
            let response = request_json(
                config,
                Method::POST,
                config.admin_url(&format!("tenants/{tenant_id}/resume")),
                None,
            )
            .await?;
            output.print_value(&response)
        }
        TenantCommands::Token { command } => handle_token_command(command, config, &output).await,
    }
}

async fn handle_token_command(
    command: TenantTokenCommands,
    config: &Config,
    output: &OutputManager,
) -> Result<()> {
    match command {
        TenantTokenCommands::Create {
            tenant_id,
            label,
            expires_at,
            token_file,
        } => {
            let response = request_json(
                config,
                Method::POST,
                config.admin_url(&format!("tenants/{tenant_id}/tokens")),
                Some(json!({ "label": label, "expires_at": expires_at })),
            )
            .await?;
            persist_issued_token(response, Path::new(&token_file), output)
        }
        TenantTokenCommands::List { tenant_id } => {
            let response = request_json(
                config,
                Method::GET,
                config.admin_url(&format!("tenants/{tenant_id}/tokens")),
                None,
            )
            .await?;
            output.print_value(&response)
        }
        TenantTokenCommands::Revoke {
            tenant_id,
            token_id,
        } => {
            let token_id = Uuid::parse_str(&token_id)
                .with_context(|| format!("Invalid token id: {token_id}"))?;
            let response = request_json(
                config,
                Method::DELETE,
                config.admin_url(&format!("tenants/{tenant_id}/tokens/{token_id}")),
                None,
            )
            .await?;
            output.print_value(&response)
        }
    }
}

async fn request_json(
    config: &Config,
    method: Method,
    url: String,
    body: Option<Value>,
) -> Result<Value> {
    let client = Client::new();
    let mut request = client
        .request(method, &url)
        .bearer_auth(&config.server.token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("Failed to request Sealbox admin API: {url}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .context("Failed to read server response")?;
    let parsed = serde_json::from_str::<Value>(&text).with_context(|| {
        format!("Sealbox returned non-JSON output with status {status}: {text}")
    })?;
    if !status.is_success() {
        anyhow::bail!("Sealbox admin API returned {status}: {parsed}");
    }
    Ok(parsed)
}

fn persist_issued_token(
    mut response: Value,
    token_file: &Path,
    output: &OutputManager,
) -> Result<()> {
    let token = response
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .context("Sealbox tenant response did not include an API token")?
        .to_string();
    write_private_new_file(token_file, &token)?;
    let object = response
        .as_object_mut()
        .context("Sealbox tenant response was not a JSON object")?;
    object.remove("token");
    object.insert(
        "token_file".to_string(),
        Value::String(token_file.display().to_string()),
    );
    output.print_value(&response)
}

fn write_private_new_file(path: &Path, token: &str) -> Result<()> {
    let path = expand_home(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create token directory: {}", parent.display()))?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    let mut file = options
        .open(&path)
        .with_context(|| format!("Refusing to overwrite token file: {}", path.display()))?;
    writeln!(file, "{token}")?;
    file.sync_all()?;
    Ok(())
}

fn expand_home(path: &Path) -> Result<PathBuf> {
    let rendered = path.to_string_lossy();
    if let Some(relative) = rendered.strip_prefix("~/") {
        return Ok(dirs::home_dir()
            .context("Unable to determine home directory")?
            .join(relative));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_private_token_file_without_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        write_private_new_file(&path, "secret-token").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "secret-token\n");
        assert!(write_private_new_file(&path, "replacement").is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn rejects_invalid_token_id_before_request() {
        assert!(Uuid::parse_str("not-a-token").is_err());
    }
}
