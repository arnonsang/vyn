use anyhow::Result;
use vyn_core::models::load_global_config;

use crate::output;
use crate::version::newer_version;

const INSTALL_URL: &str = "https://raw.githubusercontent.com/arnonsang/vyn/main/scripts/install.sh";
const RELEASES_URL: &str = "https://github.com/arnonsang/vyn/releases";
const DOCKER_IMAGE: &str = "ghcr.io/arnonsang/vyn:latest";

pub struct UpdateOptions {
    pub check_only: bool,
}

pub fn run(opts: UpdateOptions) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    let latest = newer_version(true);

    match &latest {
        Some(v) => {
            output::print_warning(&format!("vyn v{current} is installed. v{v} is available."));
        }
        None => {
            output::print_success(&format!("vyn v{current} is up to date."));
            return Ok(());
        }
    }

    if opts.check_only {
        return Ok(());
    }

    let cfg = load_global_config();
    println!();
    match cfg.install_method.as_deref() {
        Some("binary") => {
            println!("Run the following to update:");
            println!();
            println!("  curl -fsSL {INSTALL_URL} | sh");
        }
        Some("cargo") => {
            println!("Run the following to update:");
            println!();
            println!("  cargo install vyn --force");
        }
        Some("docker") => {
            println!("Run the following to update:");
            println!();
            println!("  docker pull {DOCKER_IMAGE}");
        }
        _ => {
            println!("Could not detect install method. Update manually from:");
            println!("  {RELEASES_URL}");
        }
    }

    Ok(())
}
