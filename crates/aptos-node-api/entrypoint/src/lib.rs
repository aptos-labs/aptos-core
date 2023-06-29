// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context as AnyhowContext, Result};
use aptos_config::config::{ApiConfig, NodeConfig};
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_node_api_context::Context;
use aptos_node_api_v1_core::{build_api_v1_routes, ApiV1Config};
use aptos_storage_interface::DbReader;
use aptos_types::chain_id::ChainId;
use poem::{
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    IntoEndpoint, Route, Server,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::runtime::{Handle, Runtime};

/// Create a runtime and attach the Poem webserver to it.
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> anyhow::Result<Runtime> {
    let max_runtime_workers = get_max_runtime_workers(&config.api);
    let runtime = aptos_runtimes::spawn_named_runtime("node_api".into(), Some(max_runtime_workers));

    let context = Arc::new(Context::new(chain_id, db, mp_sender, config.clone()));

    let api_v1_config = ApiV1Config::new(context);

    let routes = build_routes(api_v1_config).context("Failed to build API routes")?;

    attach_to_runtime(runtime.handle(), config, routes, false)
        .context("Failed to attach poem to runtime")?;

    Ok(runtime)
}

pub fn build_routes(api_v1_config: ApiV1Config) -> Result<impl IntoEndpoint> {
    let v1_routes = build_api_v1_routes(api_v1_config)?;
    let routes = Route::new().nest("/v1", v1_routes);
    Ok(routes)
}

/// Returns address it is running at.
pub fn attach_to_runtime(
    runtime_handle: &Handle,
    config: &NodeConfig,
    routes: impl IntoEndpoint + Send + 'static,
    random_port: bool,
) -> anyhow::Result<SocketAddr> {
    let mut address = config.api.address;

    if random_port {
        // Let the OS assign an open port.
        address.set_port(0);
    }

    // Build listener with or without TLS
    let listener = match (&config.api.tls_cert_path, &config.api.tls_key_path) {
        (Some(tls_cert_path), Some(tls_key_path)) => {
            info!("Using TLS for API");
            let cert = std::fs::read_to_string(tls_cert_path).context(format!(
                "Failed to read TLS cert from path: {}",
                tls_cert_path
            ))?;
            let key = std::fs::read_to_string(tls_key_path).context(format!(
                "Failed to read TLS key from path: {}",
                tls_key_path
            ))?;
            let rustls_certificate = RustlsCertificate::new().cert(cert).key(key);
            let rustls_config = RustlsConfig::new().fallback(rustls_certificate);
            TcpListener::bind(address).rustls(rustls_config).boxed()
        },
        _ => {
            info!("Not using TLS for API");
            TcpListener::bind(address).boxed()
        },
    };

    let acceptor = tokio::task::block_in_place(move || {
        runtime_handle
            .block_on(async move { listener.into_acceptor().await })
            .with_context(|| format!("Failed to bind Poem to address: {}", address))
    })?;

    let actual_address = &acceptor.local_addr()[0];
    let actual_address = *actual_address
        .as_socket_addr()
        .context("Failed to get socket addr from local addr for Poem webserver")?;
    runtime_handle.spawn(async move {
        Server::new_with_acceptor(acceptor)
            .run(routes)
            .await
            .map_err(anyhow::Error::msg)
    });

    info!("API server is running at {}", actual_address);

    Ok(actual_address)
}

/// Returns the maximum number of runtime workers to be given to the
/// API runtime. Defaults to 2 * number of CPU cores if not specified
/// via the given config.
fn get_max_runtime_workers(api_config: &ApiConfig) -> usize {
    api_config
        .max_runtime_workers
        .unwrap_or_else(|| num_cpus::get() * api_config.runtime_worker_multiplier)
}
