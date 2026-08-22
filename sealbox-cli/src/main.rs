mod commands;
mod config;
mod output;

use crate::commands::{
    admin_commands, audit_commands, config_commands, grant_commands, identity_commands,
    job_commands, key_commands, runner_commands, secret_commands,
};
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

    /// This machine's identity token
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
    /// Manage grants: the permitted uses of secrets
    Grant {
        #[command(subcommand)]
        command: GrantCommands,
    },
    /// Manage identities: who may call this server, and with what authority
    Identity {
        #[command(subcommand)]
        command: IdentityCommands,
    },
    /// Read the audit trail: what was attempted, by whom, and whether it was allowed
    Audit {
        /// Only this identity
        #[arg(long)]
        identity: Option<String>,
        /// Only this action, e.g. "PUT /v1/secrets/db-password"
        #[arg(long)]
        action: Option<String>,
        /// How far back: 90s, 30m, 24h, 7d, or a Unix timestamp
        #[arg(long)]
        since: Option<String>,
        /// Maximum records to return
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Run a grant: submit a job, wait, and print the result. Never a secret value.
    Run {
        /// Grant name
        grant: String,
        /// Parameters as key=value
        params: Vec<String>,
    },
    /// Rotate a secret's value through a grant. Commits only if the grant succeeds.
    Rotate {
        /// Secret key
        secret: String,
        /// The grant that makes some upstream accept the new value
        #[arg(long)]
        via: String,
        /// Store what the grant printed instead of the generated value
        #[arg(long)]
        from_output: bool,
        /// Parameters as key=value
        params: Vec<String>,
    },
    /// Execute jobs addressed to this runner. The only place a grant runs, and the only place
    /// plaintext exists outside the server.
    Runner {
        /// This runner's identity name, matching the `runner` field in the grants it executes
        #[arg(long)]
        name: String,
    },
    /// Run one command as an admin, proving it with a passkey.
    ///
    /// The session lives in this process and dies with it: an admin has no token to store, which
    /// is what stops an agent on this machine from acting as one.
    Admin {
        /// The command to run, e.g. `identity create alice --role operator`
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
    /// Claim a server that has no identities yet, creating the first admin
    Bootstrap {
        /// The value the server was started with in SEALBOX_BOOTSTRAP_TOKEN
        #[arg(long)]
        token: String,
        /// Name for the first admin identity
        #[arg(long, default_value = "admin")]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum GrantCommands {
    /// Submit a grant from a TOML file for approval. Any identity may draft one; it exists only
    /// once a human has signed for it with a passkey.
    Add {
        /// Path to the grant file
        file: String,
    },
    /// List grants. Any identity may see what it can invoke.
    List,
    /// Show one grant, including the secrets it declares
    Show {
        /// Grant name
        name: String,
    },
    /// Remove a grant. There is no update — add the replacement instead.
    Rm {
        /// Grant name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum IdentityCommands {
    /// Create an identity and print its token once
    Create {
        /// Name, unique on this server
        name: String,
        /// One of: agent, operator, admin
        #[arg(long)]
        role: String,
    },
    /// List identities. Tokens are never shown.
    List,
    /// Revoke an identity. Takes effect on its next request; nobody else is affected.
    Revoke {
        /// Name of the identity to revoke
        name: String,
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
    /// List master keys on server
    List,
    /// Rekey: re-encrypt data keys under a different master key
    Rekey {
        /// New master key ID
        #[arg(long)]
        new_key_id: String,
        /// Old master key ID
        #[arg(long)]
        old_key_id: String,
    },
    /// Check key status
    Status,
}

#[derive(Subcommand)]
enum SecretCommands {
    /// Store a secret. The value is read from stdin — never from an argument, which would put
    /// it in shell history and in `ps` output.
    Set {
        /// Secret key name
        key: String,
        /// Time to live in seconds
        #[arg(long)]
        ttl: Option<i64>,
    },
    /// Have the server generate the value. It is encrypted without ever leaving the server, and
    /// is not returned to anyone — including the caller who asked for it.
    Gen {
        /// Secret key name
        key: String,
        /// password (printable, unambiguous) or hex (raw randomness)
        #[arg(long, default_value = "password")]
        r#type: String,
        /// Length. Defaults to 32; below 16 is refused.
        #[arg(long)]
        length: Option<usize>,
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
    /// Delete secret
    Delete {
        /// Secret key name
        key: String,
        /// Version number
        #[arg(long)]
        version: i32,
    },
    /// List all secret keys. Metadata only — never values.
    List,
    /// Which grants may use a secret: everything that credential can do here
    Uses {
        /// Secret key name
        key: String,
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
    /// Export secrets to file
    Export {
        /// Output file path
        file: String,
        /// Key pattern matching
        #[arg(long)]
        keys: Option<String>,
        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
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

    if let Commands::Admin { command } = cli.command {
        let output = crate::output::OutputManager::new(config.output.format.clone());
        config.server.token = admin_commands::authenticate(&config, &output).await?;

        // Re-parsed rather than dispatched by hand, so `admin identity create …` accepts exactly
        // what `identity create …` accepts and cannot drift from it.
        let parsed = Cli::try_parse_from(std::iter::once("sealbox".to_string()).chain(command))?;
        if matches!(parsed.command, Commands::Admin { .. }) {
            anyhow::bail!("`admin admin …` is not a thing");
        }
        return dispatch(parsed.command, config).await;
    }

    dispatch(cli.command, config).await
}

async fn dispatch(command: Commands, mut config: Config) -> Result<()> {
    let config = &mut config;
    match command {
        Commands::Config { command } => config_commands::handle_command(command, config).await,
        Commands::Key { command } => key_commands::handle_command(command, config).await,
        Commands::Secret { command } => secret_commands::handle_command(command, config).await,
        Commands::Grant { command } => grant_commands::handle_command(command, config).await,
        Commands::Identity { command } => identity_commands::handle_command(command, config).await,
        Commands::Audit {
            identity,
            action,
            since,
            limit,
        } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            audit_commands::list(config, &output, identity, action, since, limit).await
        }
        Commands::Run { grant, params } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            job_commands::run(config, &output, grant, params).await
        }
        Commands::Rotate {
            secret,
            via,
            from_output,
            params,
        } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            job_commands::rotate(config, &output, secret, via, from_output, params).await
        }
        Commands::Runner { name } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            runner_commands::run(config, &output, name).await
        }
        Commands::Admin { .. } => unreachable!("handled before dispatch"),
        Commands::Bootstrap { token, name } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            identity_commands::bootstrap(config, &output, token, name).await
        }
    }
}
