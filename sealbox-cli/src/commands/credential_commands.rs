use anyhow::{Context, Result};
use reqwest::Client;
use sealbox_server::repo::SecretInfo;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    CredentialCommands,
    config::Config,
    output::OutputManager,
    password_commands::{generate_password, print_generated_password},
    secret_commands::{
        delete_secret, fetch_decrypted_secret, fetch_secret_history, save_secret_value,
    },
};

use super::input::read_secret_from_tty_or_stdin;

#[derive(Debug, Deserialize, Serialize)]
struct CredentialSecret {
    #[serde(rename = "type")]
    credential_type: String,
    username: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CredentialMetadata {
    #[serde(rename = "type")]
    credential_type: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct ListSecretsResponse {
    secrets: Vec<SecretInfo>,
}

struct SetCredentialInput {
    key: String,
    username: String,
    ttl: Option<i64>,
    generate_password: bool,
    show_password: bool,
    password_policy: crate::PasswordPolicyArgs,
}

#[derive(Debug, Default)]
struct CredentialSearchFilters {
    name: Option<String>,
    username: Option<String>,
    query: Option<String>,
}

impl CredentialSearchFilters {
    fn new(name: Option<String>, username: Option<String>, query: Option<String>) -> Self {
        Self {
            name: name.map(|filter| filter.to_lowercase()),
            username: username.map(|filter| filter.to_lowercase()),
            query: query.map(|filter| filter.to_lowercase()),
        }
    }

    fn matches(&self, credential_name: &str, username: &str) -> bool {
        let credential_name = credential_name.to_lowercase();
        let username = username.to_lowercase();

        if let Some(filter) = &self.name
            && !credential_name.contains(filter)
        {
            return false;
        }

        if let Some(filter) = &self.username
            && !username.contains(filter)
        {
            return false;
        }

        if let Some(filter) = &self.query
            && !credential_name.contains(filter)
            && !username.contains(filter)
        {
            return false;
        }

        true
    }
}

pub async fn handle_command(command: CredentialCommands, config: &Config) -> Result<()> {
    let output = OutputManager::new(config.output.format.clone());

    match command {
        CredentialCommands::Set {
            key,
            username,
            ttl,
            generate_password,
            show_password,
            password_policy,
        } => {
            set_credential(
                config,
                &output,
                SetCredentialInput {
                    key,
                    username,
                    ttl,
                    generate_password,
                    show_password,
                    password_policy,
                },
            )
            .await
        }
        CredentialCommands::Get { key, version } => {
            get_credential(config, &output, key, version).await
        }
        CredentialCommands::List {
            name,
            username,
            query,
        } => {
            list_credentials(
                config,
                &output,
                CredentialSearchFilters::new(name, username, query),
            )
            .await
        }
        CredentialCommands::Delete { key } => delete_secret(config, &output, key, None).await,
        CredentialCommands::History { key } => get_credential_history(config, &output, key).await,
    }
}

async fn set_credential(
    config: &Config,
    output: &OutputManager,
    input: SetCredentialInput,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    if input.username.trim().is_empty() {
        anyhow::bail!("Username cannot be empty");
    }

    if input.show_password && !input.generate_password {
        anyhow::bail!("--show-password can only be used with --generate-password");
    }

    if !input.generate_password && input.password_policy.has_explicit_generation_option() {
        anyhow::bail!("Password generation options require --generate-password");
    }

    let password = if input.generate_password {
        generate_password(&input.password_policy.to_policy())?
    } else {
        read_secret_from_tty_or_stdin(output, "Enter password (input will be hidden):", "password")?
    };
    if password.trim().is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    let generated_password = if input.show_password {
        Some(password.clone())
    } else {
        None
    };

    let credential = CredentialSecret {
        credential_type: "credential".to_string(),
        username: input.username.clone(),
        password,
    };
    let metadata = CredentialMetadata {
        credential_type: "credential".to_string(),
        username: input.username,
    };

    save_secret_value(
        config,
        output,
        input.key,
        serde_json::to_string(&credential).context("Failed to serialize credential")?,
        input.ttl,
        Some(serde_json::to_string(&metadata).context("Failed to serialize credential metadata")?),
    )
    .await?;

    if let Some(password) = generated_password {
        print_generated_password(output, &password)?;
    }

    Ok(())
}

async fn get_credential(
    config: &Config,
    output: &OutputManager,
    key: String,
    version: Option<i32>,
) -> Result<()> {
    let decrypted = fetch_decrypted_secret(config, &key, version).await?;
    let credential: CredentialSecret =
        serde_json::from_str(&decrypted.value).context("Secret is not a credential payload")?;

    output.print_value(&json!({
        "key": decrypted.key,
        "version": decrypted.version,
        "expires_at": decrypted.expires_at,
        "username": credential.username,
        "password": credential.password,
        "metadata": decrypted.metadata,
    }))
}

async fn list_credentials(
    config: &Config,
    output: &OutputManager,
    filters: CredentialSearchFilters,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let client = Client::new();
    let response = client
        .get(config.api_url("secrets"))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!(
            "Server returned error (status code: {}):\n{}",
            status,
            error_body
        );
    }

    let result: ListSecretsResponse = response
        .json()
        .await
        .context("Failed to parse server response")?;
    let credentials = result
        .secrets
        .into_iter()
        .filter_map(|secret| {
            let metadata = secret.metadata.as_deref()?;
            let credential_metadata: CredentialMetadata = serde_json::from_str(metadata).ok()?;
            if credential_metadata.credential_type != "credential" {
                return None;
            }
            if !filters.matches(&secret.key, &credential_metadata.username) {
                return None;
            }
            Some(json!({
                "key": secret.key,
                "version": secret.version,
                "username": credential_metadata.username,
                "expires_at": secret.expires_at,
            }))
        })
        .collect::<Vec<_>>();

    if credentials.is_empty() {
        output.print_info("No credentials found");
    } else {
        output.print_value(&json!(credentials))?;
    }

    Ok(())
}

async fn get_credential_history(
    config: &Config,
    output: &OutputManager,
    key: String,
) -> Result<()> {
    let versions = fetch_secret_history(config, &key).await?;
    let credential_versions = versions
        .into_iter()
        .filter_map(|secret| {
            let metadata = secret.metadata.as_deref()?;
            let credential_metadata: CredentialMetadata = serde_json::from_str(metadata).ok()?;
            if credential_metadata.credential_type != "credential" {
                return None;
            }
            Some(json!({
                "key": secret.key,
                "version": secret.version,
                "username": credential_metadata.username,
                "created_at": secret.created_at,
                "updated_at": secret.updated_at,
                "expires_at": secret.expires_at,
                "metadata": secret.metadata,
            }))
        })
        .collect::<Vec<_>>();

    if credential_versions.is_empty() {
        anyhow::bail!("Secret '{key}' is not a credential or has no retained credential versions");
    }

    output.print_value(&json!(credential_versions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_search_matches_name_substring_case_insensitive() {
        let filters = CredentialSearchFilters::new(Some("POSTGRES".to_string()), None, None);

        assert!(filters.matches("db/postgres", "app_user"));
    }

    #[test]
    fn test_credential_search_matches_username_substring_case_insensitive() {
        let filters = CredentialSearchFilters::new(None, Some("APP".to_string()), None);

        assert!(filters.matches("db/postgres", "app_user"));
    }

    #[test]
    fn test_credential_search_query_matches_name_or_username() {
        let name_query = CredentialSearchFilters::new(None, None, Some("postgres".to_string()));
        let username_query = CredentialSearchFilters::new(None, None, Some("app".to_string()));

        assert!(name_query.matches("db/postgres", "service_user"));
        assert!(username_query.matches("db/postgres", "app_user"));
    }

    #[test]
    fn test_credential_search_combines_specific_filters() {
        let filters = CredentialSearchFilters::new(
            Some("db/".to_string()),
            Some("app".to_string()),
            Some("postgres".to_string()),
        );

        assert!(filters.matches("db/postgres", "app_user"));
        assert!(!filters.matches("api/postgres", "app_user"));
        assert!(!filters.matches("db/postgres", "service_user"));
    }

    #[test]
    fn test_credential_search_without_filters_matches_all_credentials() {
        let filters = CredentialSearchFilters::default();

        assert!(filters.matches("db/postgres", "app_user"));
    }
}
