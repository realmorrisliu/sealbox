use anyhow::{Context, Result};
use reqwest::Client;

use crate::{config::Config, output::OutputManager};

/// Read the audit trail. Readable by every identity, including agents — concealing it protects
/// nothing an agent could not already observe.
pub async fn list(
    config: &Config,
    output: &OutputManager,
    identity: Option<String>,
    action: Option<String>,
    since: Option<String>,
    limit: Option<usize>,
) -> Result<()> {
    config
        .validate()
        .context("Configuration validation failed")?;

    // Built through `Url` so that an action like "PUT /v1/secrets/db-password" — which contains
    // both a space and slashes — is encoded correctly.
    let mut url = reqwest::Url::parse(&format!("{}/v1/audit", config.server.url))
        .context("Invalid server URL")?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(identity) = &identity {
            pairs.append_pair("identity", identity);
        }
        if let Some(action) = &action {
            pairs.append_pair("action", action);
        }
        if let Some(since) = &since {
            pairs.append_pair("since", &parse_since(since)?.to_string());
        }
        if let Some(limit) = limit {
            pairs.append_pair("limit", &limit.to_string());
        }
    }

    let response = Client::new()
        .get(url)
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {status}: {body}");
    }

    let result: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse server response")?;
    output.print_value(&result)?;
    Ok(())
}

/// Accepts `90s`, `30m`, `24h`, `7d`, or a Unix timestamp. Relative is what anyone actually
/// types when something has just gone wrong.
fn parse_since(input: &str) -> Result<i64> {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let (value, unit) = input.split_at(input.len().saturating_sub(1));

    let seconds = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        _ => {
            return input
                .parse::<i64>()
                .context("Expected a duration like 24h, or a Unix timestamp");
        }
    };
    let amount: i64 = value
        .parse()
        .with_context(|| format!("Invalid duration: {input}"))?;
    Ok(now - amount * seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_relative_and_absolute() {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        assert!((parse_since("24h").unwrap() - (now - 86400)).abs() <= 1);
        assert!((parse_since("30m").unwrap() - (now - 1800)).abs() <= 1);
        assert_eq!(parse_since("1700000000").unwrap(), 1700000000);
        assert!(parse_since("nonsense").is_err());
    }
}
