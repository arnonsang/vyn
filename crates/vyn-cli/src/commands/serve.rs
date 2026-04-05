use anyhow::Result;

pub fn run(
    relay: bool,
    port: Option<u16>,
    data_dir: Option<String>,
    s3_bucket: Option<String>,
    s3_region: Option<String>,
    s3_endpoint: Option<String>,
    s3_prefix: Option<String>,
) -> Result<()> {
    if !relay {
        anyhow::bail!("only relay mode is currently supported; use --relay");
    }

    let port = resolve_port(port)?;
    let data_dir = resolve_string(data_dir, "VYN_RELAY_DATA_DIR", "./.vyn-relay");
    let s3_bucket = resolve_optional(s3_bucket, "VYN_RELAY_S3_BUCKET");
    let s3_region = resolve_optional(s3_region, "VYN_RELAY_S3_REGION");
    let s3_endpoint = resolve_optional(s3_endpoint, "VYN_RELAY_S3_ENDPOINT");
    let s3_prefix = resolve_optional(s3_prefix, "VYN_RELAY_S3_PREFIX");

    println!("starting vyn relay on 0.0.0.0:{port} with data dir {data_dir}");
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(vyn_relay::serve_with_config(
        port,
        data_dir,
        vyn_relay::ServeConfig {
            s3_bucket,
            s3_region,
            s3_endpoint,
            s3_prefix,
        },
    ))?;
    Ok(())
}

fn resolve_port(cli_value: Option<u16>) -> Result<u16> {
    if let Some(port) = cli_value {
        return Ok(port);
    }

    if let Ok(raw) = std::env::var("VYN_RELAY_PORT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let parsed = trimmed
                .parse::<u16>()
                .map_err(|_| anyhow::anyhow!("invalid VYN_RELAY_PORT value: {trimmed}"))?;
            return Ok(parsed);
        }
    }

    Ok(50051)
}

fn resolve_string(cli_value: Option<String>, env_key: &str, default: &str) -> String {
    if let Some(value) = cli_value {
        return value;
    }

    if let Ok(value) = std::env::var(env_key) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    default.to_string()
}

fn resolve_optional(cli_value: Option<String>, env_key: &str) -> Option<String> {
    if let Some(value) = cli_value {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(value) = std::env::var(env_key) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}
