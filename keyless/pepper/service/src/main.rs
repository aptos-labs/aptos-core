// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_service::{
    accounts::{
        account_managers::{AccountRecoveryManager, AccountRecoveryManagers},
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
    utils, vuf_pub_key,
};
use aptos_logger::{error, info, warn};
use clap::Parser;
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Instant};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// A list of account recovery managers that are allowed to override the
    /// aud claim in the JWTs they issue when handling pepper requests.
    ///
    /// For example:
    /// --account-recovery-managers="https://accounts.google.com 1234567890"
    /// --account-recovery-managers="https://accounts.facebook.com 9876543210"
    #[arg(long)]
    account_recovery_managers: Vec<AccountRecoveryManager>,

    /// Disable asynchronous updates to the account recovery database.
    /// By default, async updates are enabled to avoid blocking request handlers.
    #[arg(long)]
    disable_async_db_updates: bool, // Defaults to false if not provided

    /// The Firestore database ID (required to connect to Firestore).
    /// Only required if not running in local development mode.
    #[arg(
        long,
        requires = "google_project_id",
        required_unless_present = "local_development_mode",
        conflicts_with = "local_development_mode"
    )]
    firestore_database_id: Option<String>,

    /// The Google Cloud Project ID (required to connect to Firestore).
    /// Only required if not running in local development mode.
    #[arg(
        long,
        requires = "firestore_database_id",
        required_unless_present = "local_development_mode",
        conflicts_with = "local_development_mode"
    )]
    google_project_id: Option<String>,

    /// Run the service in local development mode (uses a test account recovery database).
    /// If this flag is not provided, the service will use the Firestore account recovery database.
    #[arg(
        long,
        conflicts_with_all = ["firestore_database_id", "google_project_id"]
    )]
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
    #[arg(
        long,
        required_unless_present = "vuf_private_key_seed_hex",
        conflicts_with = "vuf_private_key_seed_hex"
    )]
    vuf_private_key_hex: Option<String>,

    /// The hex-encoded VUF private key seed (used to derive the private key if the key is not provided directly)
    #[arg(
        long,
        required_unless_present = "vuf_private_key_hex",
        conflicts_with = "vuf_private_key_hex"
    )]
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

    // Collect the account recovery managers
    info!("Collecting the account recovery managers...");
    let account_recovery_managers =
        Arc::new(AccountRecoveryManagers::new(args.account_recovery_managers));

    // Start the cached resource fetcher
    let cached_resources = resource_fetcher::start_cached_resource_fetcher(
        args.on_chain_groth16_vk_url,
        args.on_chain_keyless_config_url,
    );

    // Create the account recovery database
    let account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync> =
        if args.local_development_mode {
            warn!("Running in local development mode! Using a test account recovery database!");
            Arc::new(TestAccountRecoveryDB::new())
        } else {
            let google_project_id = args.google_project_id.expect(
                "Google Project ID must be provided when not running in local development mode!",
            );
            let firestore_database_id = args.firestore_database_id.expect(
            "Firestore Database ID must be provided when not running in local development mode!",
        );
            Arc::new(
                FirestoreAccountRecoveryDB::new(
                    google_project_id,
                    firestore_database_id,
                    args.disable_async_db_updates,
                )
                .await,
            )
        };

    // Start the JWK fetchers
    let jwk_cache = jwk_fetcher::start_jwk_fetchers();

    // Start the pepper service
    let vuf_keypair = Arc::new((vuf_public_key, Arc::new(vuf_private_key)));
    start_pepper_service(
        args.pepper_service_port,
        vuf_keypair,
        jwk_cache,
        cached_resources,
        account_recovery_managers,
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
    vuf_keypair: Arc<(String, Arc<ark_bls12_381::Fr>)>,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
    account_recovery_managers: Arc<AccountRecoveryManagers>,
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
        let account_recovery_managers = account_recovery_managers.clone();
        let account_recovery_db = account_recovery_db.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                // Start the request timer
                let request_start_time = Instant::now();

                // Get the request origin, method and request path
                let request_origin = utils::get_request_origin(&request);
                let request_method = request.method().clone();
                let request_path = request.uri().path().to_owned();

                // Clone the required Arcs for the request handler
                let vuf_keypair = vuf_keypair.clone();
                let jwk_cache = jwk_cache.clone();
                let cached_resources = cached_resources.clone();
                let account_recovery_managers = account_recovery_managers.clone();
                let account_recovery_db = account_recovery_db.clone();

                // Handle the request
                async move {
                    // Call the request handler
                    let result = request_handler::handle_request(
                        request,
                        vuf_keypair.clone(),
                        jwk_cache.clone(),
                        cached_resources.clone(),
                        account_recovery_managers.clone(),
                        account_recovery_db.clone(),
                    )
                    .await;

                    // Update the request handling metrics and logs
                    match &result {
                        Ok(response) => {
                            // Update the request handling metrics
                            metrics::update_request_handling_metrics(
                                &request_path,
                                request_method.clone(),
                                response.status(),
                                request_start_time,
                            );

                            // If the response was not successful, log the request details
                            if !response.status().is_success() {
                                warn!(
                                    "Handled request with non-successful response! Request origin: {:?}, \
                                    request path: {:?}, request method: {:?}, response status: {:?}",
                                    request_origin,
                                    request_path,
                                    request_method,
                                    response.status()
                                );
                            }
                        },
                        Err(error) => {
                            error!(
                                "Error occurred when handling request! Request origin: {:?}, \
                                request path: {:?}, request method: {:?}, Error: {:?}",
                                request_origin, request_path, request_method, error
                            );
                        },
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
