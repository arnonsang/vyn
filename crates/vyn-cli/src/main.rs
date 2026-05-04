mod commands;
mod output;
mod version;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "vyn")]
#[command(version)]
#[command(about = "Secure env/config sync CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a new vault in the current directory.
    Init { name: Option<String> },
    /// Configure storage provider and relay URL.
    Config {
        #[arg(long = "provider")]
        provider: Option<StorageProviderArg>,
        #[arg(long = "relay-url")]
        relay_url: Option<String>,
        #[arg(long = "non-interactive", default_value_t = false)]
        non_interactive: bool,
    },
    /// Encrypt and upload changed files to remote storage.
    Push,
    /// Download and decrypt files from remote storage.
    Pull,
    /// Show local changes compared to the last known manifest.
    St {
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// Show a unified diff for a file (or all changed files).
    Diff { file: Option<String> },
    /// Show push/pull history for this vault.
    History,
    /// Run health checks on the vault, identity, and relay.
    Doctor,
    /// Rotate the vault key and re-encrypt all blobs.
    Rotate,
    /// Share vault access with a GitHub user.
    Share { user: String },
    /// Check tracked files for secret patterns.
    Check,
    /// Run a command with vault secrets injected as environment variables.
    Run {
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Link this machine to an existing vault using a relay invite.
    Link { vault_id: String },
    /// Start tracking additional files in the vault.
    Add {
        #[arg(required = true, num_args = 1..)]
        paths: Vec<String>,
    },
    /// Stop tracking files in the vault.
    Del {
        #[arg(required = true, num_args = 1..)]
        paths: Vec<String>,
    },
    /// Authenticate with GitHub and register an SSH identity on the relay.
    Auth,
    /// Clone a vault from a relay onto this machine.
    Clone {
        /// Relay URL (e.g. https://relay.example.com).
        relay_url: String,
        /// Vault ID to clone.
        vault_id: String,
    },
    /// Start a self-hosted vyn relay server.
    Serve {
        #[arg(long = "relay", default_value_t = false)]
        relay: bool,
        #[arg(long = "port")]
        port: Option<u16>,
        #[arg(long = "data-dir")]
        data_dir: Option<String>,
        #[arg(long = "s3-bucket")]
        s3_bucket: Option<String>,
        #[arg(long = "s3-region")]
        s3_region: Option<String>,
        #[arg(long = "s3-endpoint")]
        s3_endpoint: Option<String>,
        #[arg(long = "s3-prefix")]
        s3_prefix: Option<String>,
    },
    /// Check for a newer version and print upgrade instructions.
    Update {
        /// Only report whether a newer version is available without showing update instructions.
        #[arg(long = "check", default_value_t = false)]
        check: bool,
    },
    /// Relay inspection commands.
    Relay {
        #[command(subcommand)]
        sub: RelaySubcommand,
    },
}

#[derive(Debug, Subcommand)]
enum RelaySubcommand {
    /// Ping relay and verify identity authentication.
    Status,
    /// List vaults and blobs visible to the authenticated identity.
    Ls {
        /// Vault to list blobs for (lists all accessible vaults if omitted).
        #[arg(long)]
        vault: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum StorageProviderArg {
    Memory,
    Relay,
}

impl StorageProviderArg {
    fn as_str(&self) -> &'static str {
        match self {
            StorageProviderArg::Memory => "memory",
            StorageProviderArg::Relay => "relay",
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Detect and persist install method on first run (best-effort).
    detect_and_store_install_method();

    // Kick off a background cache refresh. The hint appears on the next invocation.
    version::spawn_background_check();

    let result = match cli.command {
        Commands::Init { name } => commands::init::run(name),
        Commands::Config {
            provider,
            relay_url,
            non_interactive,
        } => commands::config::run(commands::config::ConfigOptions {
            provider: provider.map(|p| p.as_str().to_string()),
            relay_url,
            non_interactive,
        }),
        Commands::Push => commands::push::run(),
        Commands::Pull => commands::pull::run(),
        Commands::St { verbose } => commands::status::run(verbose),
        Commands::Diff { file } => commands::diff::run(file),
        Commands::History => commands::history::run(),
        Commands::Doctor => commands::doctor::run(),
        Commands::Rotate => commands::rotate::run(),
        Commands::Share { user } => commands::share::run(user),
        Commands::Check => commands::check::run(),
        Commands::Run { cmd } => commands::run::run(cmd),
        Commands::Link { vault_id } => commands::link::run(vault_id),
        Commands::Add { paths } => commands::add::run(paths),
        Commands::Del { paths } => commands::del::run(paths),
        Commands::Auth => commands::auth::run(),
        Commands::Clone {
            relay_url,
            vault_id,
        } => commands::clone::run(relay_url, vault_id),
        Commands::Serve {
            relay,
            port,
            data_dir,
            s3_bucket,
            s3_region,
            s3_endpoint,
            s3_prefix,
        } => commands::serve::run(
            relay,
            port,
            data_dir,
            s3_bucket,
            s3_region,
            s3_endpoint,
            s3_prefix,
        ),
        Commands::Update { check } => {
            commands::update::run(commands::update::UpdateOptions { check_only: check })
        }
        Commands::Relay { sub } => match sub {
            RelaySubcommand::Status => commands::relay::run_status(),
            RelaySubcommand::Ls { vault } => commands::relay::run_ls(vault),
        },
    };

    // After the command runs, show an update hint if a newer version is cached.
    // Only print to a TTY so piped output is not polluted.
    if console::Term::stdout().is_term()
        && let Some(latest) = version::newer_version(false)
    {
        let current = env!("CARGO_PKG_VERSION");
        eprintln!(
            "\nA new version of vyn is available: v{current} -> v{latest}. Run 'vyn update' to update."
        );
    }

    result
}

/// Detects how vyn was installed based on the current binary path and writes
/// the result to global config if not already set.
fn detect_and_store_install_method() {
    use vyn_core::models::{load_global_config, save_global_config};

    let mut cfg = load_global_config();
    if cfg.install_method.is_some() {
        return;
    }

    let method = std::env::current_exe().ok().and_then(|exe| {
        let exe_str = exe.to_string_lossy().to_lowercase();
        if exe_str.contains(".cargo/bin") || exe_str.contains(".cargo\\bin") {
            Some("cargo".to_string())
        } else if exe_str.contains("/usr/local/bin") || exe_str.contains("/.local/bin") {
            Some("binary".to_string())
        } else {
            None
        }
    });

    if method.is_some() {
        cfg.install_method = method;
        let _ = save_global_config(&cfg);
    }
}
