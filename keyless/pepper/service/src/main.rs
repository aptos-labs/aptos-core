// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_service::{
    accounts::{
        account_managers::ACCOUNT_MANAGERS,
        account_recovery_db::{
            AccountRecoveryDBInterface, FirestoreAccountRecoveryDB, TestAccountRecoveryDB,
        },
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
use aptos_logger::{info, warn};
use clap::{ArgGroup, Parser};
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use std::{convert::Infallible, net::SocketAddr, ops::Deref, sync::Arc, time::Instant};

#[derive(Parser, Debug)]
#[command(author, version, about)]
#[command(group(
    ArgGroup::new("vuf_private_key")
        .required(true) // One of the two arguments must be provided
        .multiple(false) // Only one of the two arguments should be provided at a time (i.e., not both)
        .args(&["vuf_private_key_hex", "vuf_private_key_seed_hex"]),
))]
struct Args {
    /// Run the service in local development mode (uses a test account recovery database)
    #[arg(long)]
    local_development_mode: bool, // Defaults to false if not provided

    /// The URL to fetch the on-chain keyless account configuration resource (if not provided, no fetching will be done)
    #[arg(long)]
    on_chain_groth16_vk_url: Option<String>,

    /// The URL to fetch the on-chain keyless account configuration resource (if not provided, no fetching will be done)
    #[arg(long)]
    on_chain_keyless_config_url: Option<String>,

    /// The port for the Pepper service to listen on
    #[arg(long, default_value_t = DEFAULT_PEPPER_SERVICE_PORT)]
    pepper_service_port: u16,

    /// The hex-encoded VUF private key (used directly if provided, otherwise derived from the seed)
    #[arg(long)]
    vuf_private_key_hex: Option<String>,

    /// The hex-encoded VUF private key seed (used to derive the private key if the key is not provided directly)
    #[arg(long)]
    vuf_private_key_seed_hex: Option<String>,
}

#[tokio::main]
async fn main() {
    // Fetch the command line arguments
    let args = Args::parse();

    // Start the logger
    aptos_logger::Logger::new().init();
    info!("Starting the Pepper service...");

    // Start the metrics server
    start_metrics_server();

    // Fetch the VUF public and private keypair (this will load the private key into memory)
    info!("Fetching the VUF public and private keypair for the pepper service...");
    let (vuf_public_key, vuf_private_key) = vuf_pub_key::get_pepper_service_vuf_keypair(
        args.vuf_private_key_hex,
        args.vuf_private_key_seed_hex,
    );
    info!("Retrieved the VUF public key: {:?}", vuf_public_key);

    // Start the cached resource fetcher
    let cached_resources = resource_fetcher::start_cached_resource_fetcher(
        args.on_chain_groth16_vk_url,
        args.on_chain_keyless_config_url,
    );

    let _ = ACCOUNT_MANAGERS.deref();

    // Create the account recovery database
    let account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync> =
        if args.local_development_mode {
            warn!("Running in local development mode! Using a test account recovery database!");
            Arc::new(TestAccountRecoveryDB::new())
        } else {
            Arc::new(FirestoreAccountRecoveryDB::new().await)
        };

    // Start the JWK fetchers
    let jwk_cache = jwk_fetcher::start_jwk_fetchers();

    // Start the pepper service
    let vuf_keypair = Arc::new((vuf_public_key, vuf_private_key));
    start_pepper_service(
        args.pepper_service_port,
        vuf_keypair,
        jwk_cache,
        cached_resources,
        account_recovery_db,
    )
    .await;
}

// Starts a simple metrics server
fn start_metrics_server() {
    let _handle = tokio::spawn(async move {
        info!("Starting metrics server request handler...");

        // Create a service function that handles the metrics requests
        let make_service = make_service_fn(|_conn| async {
            Ok::<_, Infallible>(service_fn(metrics::handle_metrics_request))
        });

        // Bind the socket address, and start the server
        let socket_addr = SocketAddr::from(([0, 0, 0, 0], DEFAULT_METRICS_SERVER_PORT));
        let server = Server::bind(&socket_addr).serve(make_service);
        if let Err(error) = server.await {
            panic!("Metrics server error! Error: {}", error);
        }
    });
}

// Starts the pepper service
async fn start_pepper_service(
    pepper_service_port: u16,
    vuf_keypair: Arc<(String, ark_bls12_381::Fr)>,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
    account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync>,
) {
    info!(
        "Starting the Pepper service request handler on port {}...",
        pepper_service_port
    );

    // Create the service function that handles the endpoint requests
    let make_service = make_service_fn(move |_conn| {
        // Clone the required Arcs for the service function
        let vuf_keypair = vuf_keypair.clone();
        let jwk_cache = jwk_cache.clone();
        let cached_resources = cached_resources.clone();
        let account_recovery_db = account_recovery_db.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                // Get the request start time, method and request path
                let request_start_time = Instant::now();
                let request_method = request.method().clone();
                let request_path = request.uri().path().to_owned();

                // Clone the required Arcs for the request handler
                let vuf_keypair = vuf_keypair.clone();
                let jwk_cache = jwk_cache.clone();
                let cached_resources = cached_resources.clone();
                let account_recovery_db = account_recovery_db.clone();

                // Handle the request
                async move {
                    // Call the request handler
                    let result = request_handler::handle_request(
                        request,
                        vuf_keypair.clone(),
                        jwk_cache.clone(),
                        cached_resources.clone(),
                        account_recovery_db.clone(),
                    )
                    .await;

                    // Update the request handling metrics
                    if let Ok(response) = &result {
                        metrics::update_request_handling_metrics(
                            &request_path,
                            request_method,
                            response.status(),
                            request_start_time,
                        );
                    }

                    result
                }
            }))
        }
    });

    // Bind the socket address, and start the server
    let socket_addr = SocketAddr::from(([0, 0, 0, 0], pepper_service_port));
    let server = Server::bind(&socket_addr).serve(make_service);
    if let Err(error) = server.await {
        panic!("Pepper service error! Error: {}", error);
    }
}
