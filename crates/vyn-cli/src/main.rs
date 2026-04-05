mod commands;
mod output;

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
    Init {
        name: Option<String>,
    },
    Config {
        #[arg(long = "provider")]
        provider: Option<StorageProviderArg>,
        #[arg(long = "relay-url")]
        relay_url: Option<String>,
        #[arg(long = "non-interactive", default_value_t = false)]
        non_interactive: bool,
    },
    Push,
    Pull,
    St {
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    Diff {
        file: Option<String>,
    },
    History,
    Doctor,
    Rotate,
    Share {
        user: String,
    },
    Check,
    Run {
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    Link {
        vault_id: String,
    },
    Add {
        #[arg(required = true, num_args = 1..)]
        paths: Vec<String>,
    },
    Del {
        #[arg(required = true, num_args = 1..)]
        paths: Vec<String>,
    },
    Auth,
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

    match cli.command {
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
    }
}
