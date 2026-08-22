mod commands;
mod config;
mod output;

use crate::commands::{
    admin_commands, audit_commands, config_commands, grant_commands, identity_commands,
    issuer_commands, job_commands, key_commands, runner_commands, secret_commands,
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
    /// Manage token issuers: the platforms whose signatures authenticate a workload
    Issuer {
        #[command(subcommand)]
        command: IssuerCommands,
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
        /// Read the credential from this file before every poll, rather than from configuration.
        ///
        /// For a projected ServiceAccount token: the platform signs it, rotates it, and reissues
        /// it on restart, so there is no credential of sealbox's to store anywhere.
        #[arg(long)]
        token_file: Option<String>,
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
pub enum IssuerCommands {
    /// Register a platform whose signed tokens may authenticate. Admin only.
    Add {
        /// Name to refer to it by
        name: String,
        /// The URL its tokens carry in `iss`.
        ///
        /// The field is `issuer_url`, not `url`: clap derives an argument's **id** from the field
        /// name, so a field called `url` here would collide with the global `--url` and silently
        /// take the server's address instead.
        #[arg(long)]
        issuer_url: String,
        /// File holding its JWKS — `kubectl get --raw /openid/v1/jwks` for a cluster
        #[arg(long)]
        jwks_file: String,
    },
    /// Replace an issuer's keys. This is how a signing-key rotation lands: register the JWKS
    /// holding both keys, then register it again without the old one.
    Update {
        /// Issuer name
        name: String,
        /// File holding the new JWKS
        #[arg(long)]
        jwks_file: String,
    },
    /// List registered issuers
    List,
    /// Remove an issuer. Every identity bound to it stops authenticating.
    Rm {
        /// Issuer name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum IdentityCommands {
    /// Create an identity and print its token once — or bind it to an issuer, in which case no
    /// credential is issued at all.
    Create {
        /// Name, unique on this server
        name: String,
        /// One of: agent, operator, admin, runner
        #[arg(long)]
        role: String,
        /// Bind to this registered issuer instead of issuing a token
        #[arg(long)]
        issuer: Option<String>,
        /// The exact `sub` that may act as this identity, e.g.
        /// system:serviceaccount:sealbox:runner
        #[arg(long)]
        subject: Option<String>,
        /// The `aud` its tokens must carry. Not the platform's default audience — that is the
        /// point of requiring it.
        #[arg(long)]
        audience: Option<String>,
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
        /// Time to live in seconds. This DELETES the secret when it passes — for a rotation
        /// deadline use --rotate-after.
        #[arg(long)]
        ttl: Option<i64>,
        /// How long this value should stand before it is rotated: 30d, 12h. Recorded, never
        /// acted on: `secret list --overdue` is what reads it.
        #[arg(long)]
        rotate_after: Option<String>,
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
        /// Time to live in seconds. This DELETES the secret when it passes — for a rotation
        /// deadline use --rotate-after.
        #[arg(long)]
        ttl: Option<i64>,
        /// How long this value should stand before it is rotated: 30d, 12h.
        #[arg(long)]
        rotate_after: Option<String>,
    },
    /// Show a secret's metadata: that it exists, its version, and when it last changed. Never
    /// its value, and never its ciphertext.
    Show {
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
    List {
        /// Only those past their declared rotation interval
        #[arg(long)]
        overdue: bool,
    },
    /// Which grants may use a secret: everything that credential can do here
    Uses {
        /// Secret key name
        key: String,
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
        Commands::Issuer { command } => issuer_commands::handle_command(command, config).await,
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
        Commands::Runner { name, token_file } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            runner_commands::run(config, &output, name, token_file).await
        }
        Commands::Admin { .. } => unreachable!("handled before dispatch"),
        Commands::Bootstrap { token, name } => {
            let output = crate::output::OutputManager::new(config.output.format.clone());
            identity_commands::bootstrap(config, &output, token, name).await
        }
    }
}
