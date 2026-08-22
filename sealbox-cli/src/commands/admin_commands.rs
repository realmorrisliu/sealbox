//! `sealbox admin <command>` — run one admin command behind a passkey.
//!
//! An admin has no token to hand over (ADR 0009), so authority is proved at the moment it is
//! used: this opens a sign-in the server hands back only after a signature, keeps the session in
//! this process's memory, and runs the command with it. Nothing is written to disk, and the
//! session is never printed — a credential a human copies is a credential that lands in
//! scrollback.

use anyhow::{Context, Result};
use reqwest::Client;

use crate::{config::Config, output::OutputManager};

/// How long to wait for someone to reach for their phone before giving up.
const WAIT: std::time::Duration = std::time::Duration::from_secs(180);

pub async fn authenticate(config: &Config, output: &OutputManager) -> Result<String> {
    config
        .validate()
        .context("Configuration validation failed")?;

    let client = Client::new();
    let opened: serde_json::Value = client
        .post(format!("{}/v1/auth/login", config.server.url))
        .send()
        .await
        .context("Failed to request server")?
        .error_for_status()
        .context("Server refused to open a sign-in")?
        .json()
        .await
        .context("Failed to parse server response")?;

    let (id, url) = (
        opened["login"].as_str().unwrap_or_default().to_string(),
        opened["url"].as_str().unwrap_or_default().to_string(),
    );

    // Printed as well as opened: approving on a phone is not a fallback for a missing browser,
    // it is the better arrangement — the machine running the agent is not the machine that
    // decides.
    output.print_info(&format!("Sign in with your passkey:\n  {url}"));
    output.print_info("  (open it here, or on your phone — the terminal will pick it up)");
    let _ = open_in_browser(&url);

    let deadline = std::time::Instant::now() + WAIT;
    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!("Timed out waiting for the sign-in to be approved");
        }
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        let body: serde_json::Value = client
            .get(format!("{}/v1/auth/login/{id}", config.server.url))
            .send()
            .await
            .context("Failed to request server")?
            .json()
            .await
            .context("Failed to parse server response")?;

        if let Some(session) = body["session"].as_str() {
            output.print_success("Signed in.");
            return Ok(session.to_string());
        }
    }
}

/// Best-effort. A headless machine has no browser, and printing the URL is the real interface.
pub fn open_in_browser(url: &str) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}
