mod commands;
mod config;
mod output;

use crate::commands::{config_commands, key_commands, secret_commands};
use crate::config::{Config, OutputFormat};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sealbox")]
#[command(author = "Sealbox Team")]
#[command(version = "1.0.0")]
#[command(about = "Sealbox CLI - End-to-end encrypted secret management tool")]
#[command(
    long_about = "Sealbox is a lightweight, single-node secret storage service with end-to-end encryption using RSA key pairs."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL
    #[arg(long, global = true)]
    url: Option<String>,

    /// Authentication token
    #[arg(long, global = true)]
    token: Option<String>,

    /// Public key file path
    #[arg(long, global = true)]
    public_key: Option<String>,

    /// Private key file path
    #[arg(long, global = true)]
    private_key: Option<String>,

    /// Output format
    #[arg(long, global = true, value_enum)]
    output: Option<OutputFormatArg>,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormatArg {
    Json,
    Yaml,
    Table,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Yaml => OutputFormat::Yaml,
            OutputFormatArg::Table => OutputFormat::Table,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Manage keys
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },
    /// Manage secrets
    Secret {
        #[command(subcommand)]
        command: SecretCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key (e.g., server.url, server.token, keys.public_key_path)
        key: String,
        /// Configuration value
        value: String,
    },
    /// Initialize configuration
    Init {
        /// Server URL
        #[arg(long)]
        url: Option<String>,
        /// Authentication token
        #[arg(long)]
        token: Option<String>,
        /// Public key file path
        #[arg(long)]
        public_key: Option<String>,
        /// Private key file path
        #[arg(long)]
        private_key: Option<String>,
        /// Output format
        #[arg(long, value_enum)]
        output: Option<OutputFormatArg>,
        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum KeyCommands {
    /// Generate new key pair
    Generate {
        /// Public key file path
        #[arg(long)]
        public_key_path: Option<String>,
        /// Private key file path
        #[arg(long)]
        private_key_path: Option<String>,
        /// Overwrite existing key files
        #[arg(long)]
        force: bool,
    },
    /// Register public key to server
    Register,
    /// Rotate client key
    Rotate {
        /// New client key ID
        #[arg(long)]
        new_key_id: String,
        /// Old client key ID
        #[arg(long)]
        old_key_id: String,
    },
    /// Check key status
    Status,
}

#[derive(Subcommand)]
enum SecretCommands {
    /// Set secret
    Set {
        /// Secret key name
        key: String,
        /// Secret value (read from stdin if not provided)
        value: Option<String>,
        /// Time to live in seconds
        #[arg(long)]
        ttl: Option<i64>,
    },
    /// Get secret
    Get {
        /// Secret key name
        key: String,
        /// Specific version number
        #[arg(long)]
        version: Option<i32>,
    },
    /// View secret version history
    History {
        /// Secret key name
        key: String,
    },
    /// Import secrets from file
    Import {
        /// Input file path
        file: String,
        /// File format
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Export secrets to file or stdout in various formats
    Export {
        /// Output file path (use "-" for stdout)
        #[arg(default_value = "-")]
        file: String,
        /// Key pattern matching (supports glob patterns)
        #[arg(long)]
        keys: Option<String>,
        /// Output format (json, yaml, env, shell)
        #[arg(long, default_value = "env")]
        format: String,
        /// Prefix for environment variable names
        #[arg(long)]
        prefix: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let mut config = Config::load()?;

    // Command line arguments override configuration
    if let Some(url) = cli.url {
        config.server.url = url;
    }
    if let Some(token) = cli.token {
        config.server.token = token;
    }
    if let Some(public_key) = cli.public_key {
        config.keys.public_key_path = public_key.into();
    }
    if let Some(private_key) = cli.private_key {
        config.keys.private_key_path = private_key.into();
    }
    if let Some(output) = cli.output {
        config.output.format = output.into();
    }

    // Execute command
    match cli.command {
        Commands::Config { command } => config_commands::handle_command(command, &mut config).await,
        Commands::Key { command } => key_commands::handle_command(command, &config).await,
        Commands::Secret { command } => secret_commands::handle_command(command, &config).await,
    }
}
