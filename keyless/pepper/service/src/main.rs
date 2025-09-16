// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_service::{
    accounts::{
        account_db::{init_account_db, ACCOUNT_RECOVERY_DB},
        account_managers::ACCOUNT_MANAGERS,
    },
    external_resources::{
        jwk_fetcher, jwk_fetcher::JWKCache, resource_fetcher, resource_fetcher::CachedResources,
    },
    metrics,
    metrics::DEFAULT_METRICS_SERVER_PORT,
    request_handler,
    request_handler::DEFAULT_PEPPER_SERVICE_PORT,
    vuf_pub_key,
};
use aptos_logger::info;
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use std::{convert::Infallible, net::SocketAddr, ops::Deref, sync::Arc};

#[tokio::main]
async fn main() {
    // Start the logger
    aptos_logger::Logger::new().init();
    info!("Starting the Pepper service...");

    // Start the metrics server
    start_metrics_server();

    // Fetch the VUF public and private keypair (this will load the private key into memory)
    info!("Fetching the VUF public and private keypair for the pepper service...");
    let (vuf_public_key, vuf_private_key) = vuf_pub_key::get_pepper_service_vuf_keypair();
    info!("Retrieved the VUF public key: {:?}", vuf_public_key);

    // Start the cached resource fetcher
    let cached_resources = resource_fetcher::start_cached_resource_fetcher();

    // Initialize the account recovery database
    let _ = ACCOUNT_MANAGERS.deref();
    {
        let _db = ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await;
    }

    // Start the JWK fetchers
    let jwk_cache = jwk_fetcher::start_jwk_fetchers();

    // Start the pepper service
    let vuf_keypair = Arc::new((vuf_public_key, vuf_private_key));
    start_pepper_service(vuf_keypair, jwk_cache, cached_resources).await;
}

// Starts a simple metrics server
fn start_metrics_server() {
    let _handle = tokio::spawn(async move {
        info!("Starting metrics server request handler...");

        // Create a service function that handles the metrics requests
        let make_service = make_service_fn(|_conn| async {
            Ok::<_, Infallible>(service_fn(metrics::handle_request))
        });

        // Bind the socket address, and start the server
        let socket_addr = SocketAddr::from(([0, 0, 0, 0], DEFAULT_METRICS_SERVER_PORT));
        let server = Server::bind(&socket_addr).serve(make_service);
        if let Err(error) = server.await {
            eprintln!("Metrics server error! Error: {}", error);
        }
    });
}

// Starts the pepper service
async fn start_pepper_service(
    vuf_keypair: Arc<(String, ark_bls12_381::Fr)>,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
) {
    info!(
        "Starting the Pepper service request handler on port {}...",
        DEFAULT_PEPPER_SERVICE_PORT
    );

    // Create the service function that handles the endpoint requests
    let make_service = make_service_fn(move |_conn| {
        let vuf_keypair = vuf_keypair.clone();
        let jwk_cache = jwk_cache.clone();
        let cached_resources = cached_resources.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                request_handler::handle_request(
                    request,
                    vuf_keypair.clone(),
                    jwk_cache.clone(),
                    cached_resources.clone(),
                )
            }))
        }
    });

    // Bind the socket address, and start the server
    let socket_addr = SocketAddr::from(([0, 0, 0, 0], DEFAULT_PEPPER_SERVICE_PORT));
    let server = Server::bind(&socket_addr).serve(make_service);
    if let Err(error) = server.await {
        eprintln!("Pepper service error! Error: {}", error);
    }
}
