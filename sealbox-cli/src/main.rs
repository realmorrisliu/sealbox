mod commands;
mod config;
mod output;

use crate::commands::{
    config_commands, credential_commands, key_commands, password_commands, secret_commands,
    tenant_commands,
};
use crate::config::{Config, OutputFormat, normalize_api_version};
use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sealbox")]
#[command(author = "Sealbox Team")]
#[command(version = "1.0.0")]
#[command(about = "Sealbox CLI - client-encrypted secret management tool")]
#[command(
    long_about = "Sealbox is a lightweight, single-node secret storage service where the CLI encrypts secrets locally using RSA key pairs."
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

    /// API version for secret/key operations (v1 or v2)
    #[arg(long, global = true)]
    api_version: Option<String>,

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
    /// Manage username/password credentials
    Credential {
        #[command(subcommand)]
        command: CredentialCommands,
    },
    /// Generate strong passwords
    Password {
        #[command(subcommand)]
        command: PasswordCommands,
    },
    /// Administer isolated tenants and tenant API tokens
    Tenant {
        #[command(subcommand)]
        command: TenantCommands,
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
    /// Rotate master key
    Rotate {
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
    /// Delete secret
    Delete {
        /// Secret key name
        key: String,
        /// Specific version number. If omitted, all versions are deleted.
        #[arg(long)]
        version: Option<i32>,
    },
    /// List all secret keys (requires server support)
    List,
    /// View secret version history
    History {
        /// Secret key name
        key: String,
    },
    /// Import secrets from an encrypted Sealbox archive
    Import {
        /// Encrypted archive file path
        file: String,
        /// File format
        #[arg(long, default_value = "encrypted-tar")]
        format: String,
    },
    /// Export secrets to an encrypted Sealbox archive
    Export {
        /// Encrypted archive file path
        file: String,
        /// Key substring filter
        #[arg(long)]
        keys: Option<String>,
        /// File format
        #[arg(long, default_value = "encrypted-tar")]
        format: String,
    },
}

#[derive(Args, Clone, Debug)]
struct PasswordPolicyArgs {
    /// Password length (default: 24)
    #[arg(long)]
    length: Option<usize>,
    /// Generate only ASCII letters and digits
    #[arg(long)]
    alphanumeric: bool,
    /// Exclude symbol characters
    #[arg(long)]
    no_symbols: bool,
    /// Exclude number characters
    #[arg(long)]
    no_numbers: bool,
    /// Exclude uppercase letters
    #[arg(long)]
    no_uppercase: bool,
    /// Exclude lowercase letters
    #[arg(long)]
    no_lowercase: bool,
    /// Exclude ambiguous characters such as O, 0, I, l, and 1
    #[arg(long)]
    exclude_ambiguous: bool,
}

#[derive(Subcommand)]
enum PasswordCommands {
    /// Generate a strong password locally
    Generate {
        /// Number of passwords to generate
        #[arg(long, default_value_t = 1)]
        count: usize,
        #[command(flatten)]
        policy: PasswordPolicyArgs,
    },
}

#[derive(Subcommand)]
enum CredentialCommands {
    /// Store a username/password credential
    Set {
        /// Credential key name
        key: String,
        /// Username stored as searchable plaintext metadata and encrypted value data
        #[arg(long)]
        username: String,
        /// Time to live in seconds
        #[arg(long)]
        ttl: Option<i64>,
        /// Generate a strong password locally instead of prompting or reading stdin
        #[arg(long)]
        generate_password: bool,
        /// Print the generated password after it is saved
        #[arg(long)]
        show_password: bool,
        #[command(flatten)]
        password_policy: PasswordPolicyArgs,
    },
    /// Retrieve and decrypt a credential
    Get {
        /// Credential key name
        key: String,
        /// Specific version number
        #[arg(long)]
        version: Option<i32>,
    },
    /// List credentials using plaintext metadata
    List {
        /// Filter by credential name/key substring
        #[arg(long, visible_alias = "key")]
        name: Option<String>,
        /// Filter by username substring
        #[arg(long)]
        username: Option<String>,
        /// Filter by credential name/key or username substring
        #[arg(long)]
        query: Option<String>,
    },
    /// View credential version history
    History {
        /// Credential key name
        key: String,
    },
    /// Delete a credential and all stored versions
    Delete {
        /// Credential key name
        key: String,
    },
}

#[derive(Subcommand)]
enum TenantCommands {
    /// Create a tenant and write its initial API token to a private file
    Create {
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long)]
        token_label: Option<String>,
        #[arg(long)]
        token_expires_at: Option<i64>,
        #[arg(long)]
        token_file: String,
    },
    /// List tenant metadata
    List,
    /// Get one tenant
    Get { tenant_id: String },
    /// Suspend tenant data-token authentication
    Suspend { tenant_id: String },
    /// Resume tenant data-token authentication
    Resume { tenant_id: String },
    /// Manage tenant API tokens
    Token {
        #[command(subcommand)]
        command: TenantTokenCommands,
    },
}

#[derive(Subcommand)]
enum TenantTokenCommands {
    /// Create a tenant token and write it to a private file
    Create {
        tenant_id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        expires_at: Option<i64>,
        #[arg(long)]
        token_file: String,
    },
    /// List non-secret token metadata for a tenant
    List { tenant_id: String },
    /// Revoke a tenant token
    Revoke { tenant_id: String, token_id: String },
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
    if let Some(api_version) = cli.api_version {
        config.server.api_version = normalize_api_version(&api_version)?;
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
        Commands::Credential { command } => {
            credential_commands::handle_command(command, &config).await
        }
        Commands::Password { command } => password_commands::handle_command(command, &config).await,
        Commands::Tenant { command } => tenant_commands::handle_command(command, &config).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_password_generate_alphanumeric() {
        let cli = Cli::try_parse_from([
            "sealbox",
            "password",
            "generate",
            "--alphanumeric",
            "--length",
            "40",
        ])
        .unwrap();

        match cli.command {
            Commands::Password {
                command:
                    PasswordCommands::Generate {
                        count,
                        policy:
                            PasswordPolicyArgs {
                                length,
                                alphanumeric,
                                ..
                            },
                    },
            } => {
                assert_eq!(count, 1);
                assert_eq!(length, Some(40));
                assert!(alphanumeric);
            }
            _ => panic!("Expected password generate command"),
        }
    }

    #[test]
    fn test_parse_credential_set_generate_password() {
        let cli = Cli::try_parse_from([
            "sealbox",
            "credential",
            "set",
            "db/postgres",
            "--username",
            "app_user",
            "--generate-password",
            "--alphanumeric",
            "--show-password",
        ])
        .unwrap();

        match cli.command {
            Commands::Credential {
                command:
                    CredentialCommands::Set {
                        key,
                        username,
                        generate_password,
                        show_password,
                        password_policy,
                        ..
                    },
            } => {
                assert_eq!(key, "db/postgres");
                assert_eq!(username, "app_user");
                assert!(generate_password);
                assert!(show_password);
                assert!(password_policy.alphanumeric);
            }
            _ => panic!("Expected credential set command"),
        }
    }

    #[test]
    fn test_parse_credential_list_search_filters() {
        let cli = Cli::try_parse_from([
            "sealbox",
            "credential",
            "list",
            "--name",
            "db/",
            "--username",
            "app",
            "--query",
            "prod",
        ])
        .unwrap();

        match cli.command {
            Commands::Credential {
                command:
                    CredentialCommands::List {
                        name,
                        username,
                        query,
                    },
            } => {
                assert_eq!(name.as_deref(), Some("db/"));
                assert_eq!(username.as_deref(), Some("app"));
                assert_eq!(query.as_deref(), Some("prod"));
            }
            _ => panic!("Expected credential list command"),
        }
    }

    #[test]
    fn test_parse_secret_delete_without_version() {
        let cli = Cli::try_parse_from(["sealbox", "secret", "delete", "db/postgres"]).unwrap();

        match cli.command {
            Commands::Secret {
                command: SecretCommands::Delete { key, version },
            } => {
                assert_eq!(key, "db/postgres");
                assert_eq!(version, None);
            }
            _ => panic!("Expected secret delete command"),
        }
    }

    #[test]
    fn test_parse_secret_delete_with_version() {
        let cli = Cli::try_parse_from([
            "sealbox",
            "secret",
            "delete",
            "db/postgres",
            "--version",
            "2",
        ])
        .unwrap();

        match cli.command {
            Commands::Secret {
                command: SecretCommands::Delete { key, version },
            } => {
                assert_eq!(key, "db/postgres");
                assert_eq!(version, Some(2));
            }
            _ => panic!("Expected secret delete command"),
        }
    }

    #[test]
    fn test_parse_credential_delete() {
        let cli = Cli::try_parse_from(["sealbox", "credential", "delete", "db/postgres"]).unwrap();

        match cli.command {
            Commands::Credential {
                command: CredentialCommands::Delete { key },
            } => {
                assert_eq!(key, "db/postgres");
            }
            _ => panic!("Expected credential delete command"),
        }
    }

    #[test]
    fn test_parse_credential_history() {
        let cli = Cli::try_parse_from(["sealbox", "credential", "history", "db/postgres"]).unwrap();

        match cli.command {
            Commands::Credential {
                command: CredentialCommands::History { key },
            } => {
                assert_eq!(key, "db/postgres");
            }
            _ => panic!("Expected credential history command"),
        }
    }
}
