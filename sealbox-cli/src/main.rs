use anyhow::Result;
use clap::{Parser, Subcommand};
use reqwest::Client;
use rsa::pkcs1::DecodeRsaPublicKey;
use serde_json::json;
use std::fs;
use std::path::Path;
use time::format_description::well_known::Rfc2822;
use time::{OffsetDateTime, UtcOffset};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage master keys
    MasterKey {
        #[command(subcommand)]
        command: MasterKeyCommands,
    },
}

#[derive(Subcommand)]
enum MasterKeyCommands {
    /// Create a new master key on the server.
    /// If key files are not found at the specified paths, a new key pair will be generated.
    Create {
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        url: String,
        #[arg(long)]
        token: String,
        #[arg(long, default_value = "public_key.pem")]
        public_key_path: String,
        #[arg(long, default_value = "private_key.pem")]
        private_key_path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::MasterKey { command } => match command {
            MasterKeyCommands::Create {
                url,
                token,
                public_key_path,
                private_key_path,
            } => {
                // Check if keypair exists, generate if not
                if !Path::new(public_key_path).exists() || !Path::new(private_key_path).exists() {
                    println!("Key pair not found. Generating a new one...");
                    let (private_key, public_key) =
                        sealbox_server::crypto::generate_master_key_pair()?;

                    fs::write(private_key_path, private_key)?;
                    fs::write(public_key_path, public_key)?;

                    println!("Key pair created successfully.");
                    println!("Private key saved to: {}", private_key_path);
                    println!("Public key saved to: {}", public_key_path);
                }

                let public_key_pem = fs::read_to_string(public_key_path)?;

                // Validate the key format before sending
                if let Err(_) = rsa::RsaPublicKey::from_pkcs1_pem(&public_key_pem) {
                    anyhow::bail!(
                        "Invalid public key format in file: {}. Please ensure it is a valid PKCS#1 PEM-encoded public key.",
                        public_key_path
                    );
                }

                println!("Registering public key with the server...");

                let client = Client::new();
                let res = client
                    .post(format!("{}/v1/master-key", url))
                    .bearer_auth(token)
                    .json(&json!({ "public_key": public_key_pem }))
                    .send()
                    .await?;

                let status = res.status();
                let body = res.text().await?;

                if status.is_success() {
                    let master_key: sealbox_server::repo::MasterKey = serde_json::from_str(&body)?;
                    let created_at = OffsetDateTime::from_unix_timestamp(master_key.created_at)?;
                    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
                    let created_at_local = created_at.to_offset(local_offset);

                    println!("Master key registered successfully!");
                    println!("  ID: {}", master_key.id);
                    println!("  Status: {:?}", master_key.status);
                    println!(
                        "  Created At: {} (Local Time)",
                        created_at_local
                            .format(&Rfc2822)
                            .unwrap_or(created_at_local.to_string())
                    );
                } else {
                    println!(
                        "
Server returned an error ({})",
                        status
                    );
                    println!("{}", body);
                }
            }
        },
    }

    Ok(())
}
