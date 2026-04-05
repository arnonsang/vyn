pub mod auth;
pub mod service;
pub mod store;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use tonic::transport::Server;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

pub mod proto {
    tonic::include_proto!("vyn");
}

#[derive(Debug, Clone, Default)]
pub struct ServeConfig {
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_prefix: Option<String>,
}

pub async fn serve(port: u16, data_dir: String) -> Result<()> {
    serve_with_config(port, data_dir, ServeConfig::default()).await
}

pub async fn serve_with_config(port: u16, data_dir: String, config: ServeConfig) -> Result<()> {
    let backend_mode = if config.s3_bucket.is_some() && config.s3_region.is_some() {
        "local + s3-mirror"
    } else {
        "local-only"
    };

    if config.s3_bucket.is_some() ^ config.s3_region.is_some() {
        eprintln!(
            "relay warning: partial S3 config detected (need both --s3-bucket and --s3-region); running local-only"
        );
    }

    println!("relay startup: backend_mode={backend_mode}, data_dir={data_dir}, port={port}");

    let store = store::FileStore::new(&data_dir, config)
        .await
        .context("failed to initialize relay store backend")?;
    store.init().context("failed to initialize relay store")?;

    let addr: SocketAddr = format!("0.0.0.0:{port}")
        .parse()
        .context("failed to parse relay bind address")?;

    let service = service::RelayService::new(store);

    // TLS: to enable TLS, use Server::builder().tls_config(...) before add_service,
    // or terminate TLS at a reverse proxy (nginx, caddy) in front of this server.
    Server::builder()
        .add_service(proto::vyn_relay_server::VynRelayServer::new(service))
        .serve(addr)
        .await
        .context("relay server terminated with an error")
}

/// Start the relay server on an already-bound listener. Used in tests to avoid
/// the TOCTOU race between binding a port and handing it to tonic.
pub async fn serve_with_listener(listener: TcpListener, data_dir: String) -> Result<()> {
    let store = store::FileStore::new(&data_dir, ServeConfig::default())
        .await
        .context("failed to initialize relay store backend")?;
    store.init().context("failed to initialize relay store")?;

    let service = service::RelayService::new(store);

    Server::builder()
        .add_service(proto::vyn_relay_server::VynRelayServer::new(service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .context("relay server terminated with an error")
}
