// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::constant_time;
use aptos_keyless_pepper_common::PepperInput;
use aptos_keyless_pepper_service::{
    accounts::{
        account_managers::{AccountRecoveryManager, AccountRecoveryManagers},
        account_recovery_db::{
            AccountRecoveryDBInterface, FirestoreAccountRecoveryDB, TestAccountRecoveryDB,
        },
    },
    dedicated_handlers::pepper_request,
    deployment_information::DeploymentInformation,
    external_resources::{
        jwk_fetcher,
        jwk_types::{FederatedJWKIssuer, FederatedJWKs, JWKCache, JWKIssuer},
        resource_fetcher,
        resource_fetcher::CachedResources,
    },
    metrics,
    metrics::DEFAULT_METRICS_SERVER_PORT,
    request_handler,
    request_handler::DEFAULT_PEPPER_SERVICE_PORT,
    utils, vuf_keypair,
    vuf_keypair::VUFKeypair,
};
use aptos_logger::{error, info, warn};
use clap::Parser;
use dudect_bencher::{ctbench, ctbench::BenchName};
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use more_asserts::assert_le;
use num_traits::ToPrimitive;
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Instant};

// The key used to store the derived pepper in the deployment information
const DERIVED_PEPPER_ON_STARTUP_KEY: &str = "derived_pepper_on_startup";

// Constants for deriving the fixed pepper on startup. These are
// hardcoded to ensure that the pepper derivation logic remains
// consistent and backwards compatible across deployments.
const FIXED_PEPPER_INPUT_ISS: &str = "fixed_issuer";
const FIXED_PEPPER_INPUT_UID_KEY: &str = "fixed_sub";
const FIXED_PEPPER_INPUT_UID_VAL: &str = "fixed_user_id";
const FIXED_PEPPER_INPUT_AUD: &str = "fixed_audience";

// The field name in the VUF JSON that contains the public key
const PUBLIC_KEY_FIELD_NAME: &str = "public_key";

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

    /// If provided, the pepper service will compute a locally derived pepper
    /// and compare it against this expected (hex encoded) value. This helps to
    /// ensure that the pepper derivation logic is consistent and backwards
    /// compatible. If no value is provided, no verification will be done.
    #[arg(long)]
    expected_derived_pepper_on_startup: Option<String>,

    /// If provided, the pepper service will verify that the VUF public key
    /// matches this expected (hex encoded) value on startup. This helps to
    /// ensure that the correct VUF keypair is being used by the service. If
    /// no value is provided, no verification will be done.
    #[arg(long)]
    expected_vuf_pubkey_on_startup: Option<String>,

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

    /// A list of JWK URLs the pepper service should monitor in addition to the default issuers.
    /// This can also be used to override the JWK URL of default issuers.
    ///
    /// For example:
    /// --jwk-issuers-override="https://accounts.google.com https://www.googleapis.com/oauth2/v999/certs"
    /// --jwk-issuers-override="https://www.facebook.com https://www.facebook.com/.well-known/oauth/openid/jwks"
    #[arg(long)]
    jwk_issuers_override: Vec<JWKIssuer>,
}

/// The DudeCT statistical test must output a `max_t` value whose absolute value is <= to this.
///
/// Docs here: https://docs.rs/dudect-bencher/latest/dudect_bencher/
/// Original paper here: https://eprint.iacr.org/2016/1123.pdf
const ABS_MAX_T: i64 = 5;

#[tokio::main]
async fn main() {
    // Fetch the command line arguments
    let args = Args::parse();

    // Start the logger
    aptos_logger::Logger::new().init();
    info!("Starting the Pepper service...");

    // Start the metrics server
    start_metrics_server();

    // Get the deployment information
    let deployment_information = DeploymentInformation::new();

    // Fetch the VUF public and private keypair (this will load the private key into memory)
    info!("Fetching the VUF public and private keypair for the pepper service...");
    let vuf_keypair = Arc::new(vuf_keypair::get_pepper_service_vuf_keypair(
        args.vuf_private_key_hex.clone(),
        args.vuf_private_key_seed_hex.clone(),
    ));
    info!(
        "Retrieved the VUF public key: {:?}",
        vuf_keypair.vuf_public_key_json()
    );

    // Verify the critical service invariants
    info!("Verifying critical service invariants...");
    verify_critical_service_invariants(&args, vuf_keypair.clone(), deployment_information.clone());

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
    let (jwk_cache, federated_jwks) =
        jwk_fetcher::start_jwk_fetchers(args.jwk_issuers_override.clone());

    // Start the pepper service
    start_pepper_service(
        args.pepper_service_port,
        vuf_keypair,
        jwk_cache,
        federated_jwks,
        cached_resources,
        account_recovery_managers,
        account_recovery_db,
        deployment_information,
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
    vuf_keypair: Arc<VUFKeypair>,
    jwk_cache: JWKCache,
    federated_jwks: FederatedJWKs<FederatedJWKIssuer>,
    cached_resources: CachedResources,
    account_recovery_managers: Arc<AccountRecoveryManagers>,
    account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync>,
    deployment_information: DeploymentInformation,
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
        let federated_jwks = federated_jwks.clone();
        let cached_resources = cached_resources.clone();
        let account_recovery_managers = account_recovery_managers.clone();
        let account_recovery_db = account_recovery_db.clone();
        let deployment_information = deployment_information.clone();

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
                let federated_jwks = federated_jwks.clone();
                let cached_resources = cached_resources.clone();
                let account_recovery_managers = account_recovery_managers.clone();
                let account_recovery_db = account_recovery_db.clone();
                let deployment_information = deployment_information.clone();

                // Handle the request
                async move {
                    // Call the request handler
                    let result = request_handler::handle_request(
                        request,
                        vuf_keypair.clone(),
                        jwk_cache.clone(),
                        federated_jwks.clone(),
                        cached_resources.clone(),
                        account_recovery_managers.clone(),
                        account_recovery_db.clone(),
                        deployment_information.clone(),
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

/// Verifies that scalar multiplication is constant time
fn verify_constant_time_scalar_multiplication() {
    // Run the constant time benchmarks for random bases
    let abs_max_t = ctbench::run_bench(
        &BenchName("blstrs_scalar_mul/random_bases"),
        constant_time::blstrs_scalar_mul::run_bench_with_random_bases,
        None,
    )
    .1
    .max_t
    .abs()
    .ceil()
    .to_i64()
    .expect("Floating point arithmetic went awry.");
    assert_le!(abs_max_t, ABS_MAX_T);

    // Run the constant time benchmarks for fixed bases
    let abs_max_t = ctbench::run_bench(
        &BenchName("blstrs_scalar_mul/fixed_bases"),
        constant_time::blstrs_scalar_mul::run_bench_with_fixed_bases,
        None,
    )
    .1
    .max_t
    .abs()
    .ceil()
    .to_i64()
    .expect("Floating point arithmetic went awry.");
    assert_le!(abs_max_t, ABS_MAX_T);
}

/// Verifies critical service invariants. If any of the invariants fail,
/// this function will panic. This helps to ensure that no critical properties
/// are violated during development, or after deployment.
fn verify_critical_service_invariants(
    args: &Args,
    vuf_keypair: Arc<VUFKeypair>,
    deployment_information: DeploymentInformation,
) {
    // Verify constant-time scalar multiplication if in production.
    if args.local_development_mode {
        info!(
            "Constant-time scalar multiplication verification skipped in local development mode."
        );
    } else {
        info!("Verifying constant-time scalar multiplication...");
        verify_constant_time_scalar_multiplication();
    }

    // Verify the VUF public key
    if let Some(expected_vuf_pubkey) = args.expected_vuf_pubkey_on_startup.as_ref() {
        info!("Verifying expected VUF public key...");
        verify_expected_vuf_public_key(vuf_keypair.clone(), expected_vuf_pubkey);
    } else {
        warn!("No expected VUF public key provided for startup verification!");
    }

    // Verify the expected derived pepper
    if let Some(expected_derived_pepper) = args.expected_derived_pepper_on_startup.as_ref() {
        info!("Verifying expected derived pepper...");
        verify_expected_derived_pepper(
            vuf_keypair,
            expected_derived_pepper,
            deployment_information,
        );
    } else {
        warn!("No expected derived pepper provided for startup verification!");
    }
}

/// Verifies that the locally derived pepper matches the expected
/// value, and if so, inserts it into the deployment information.
/// This is useful for observability and debugging purposes.
fn verify_expected_derived_pepper(
    vuf_keypair: Arc<VUFKeypair>,
    expected_derived_pepper: &str,
    mut deployment_information: DeploymentInformation,
) {
    // Create the pepper input
    let pepper_input = PepperInput {
        iss: FIXED_PEPPER_INPUT_ISS.into(),
        uid_key: FIXED_PEPPER_INPUT_UID_KEY.into(),
        uid_val: FIXED_PEPPER_INPUT_UID_VAL.into(),
        aud: FIXED_PEPPER_INPUT_AUD.into(),
    };

    // Derive the pepper locally and hex encode it
    let (_, derived_pepper_bytes, _) =
        pepper_request::derive_pepper_and_account_address(vuf_keypair, None, &pepper_input)
            .unwrap();
    let derived_pepper = hex::encode(derived_pepper_bytes);

    // Verify the derived pepper matches the expected value
    if derived_pepper != expected_derived_pepper {
        panic!(
            "The derived pepper does not match the expected value! Derived: {:?}, Expected: {:?}",
            derived_pepper, expected_derived_pepper
        );
    }

    // Insert the derived pepper into the deployment information
    deployment_information
        .extend_deployment_information(DERIVED_PEPPER_ON_STARTUP_KEY.into(), derived_pepper);
}

/// Verifies that the local public key matches the expected value
fn verify_expected_vuf_public_key(vuf_keypair: Arc<VUFKeypair>, expected_vuf_public_key: &str) {
    // Get the public key as a JSON value
    let vuf_public_key_json = vuf_keypair.vuf_public_key_json();
    let vuf_public_key_value = serde_json::from_str::<serde_json::Value>(vuf_public_key_json)
        .expect("Failed to parse VUF public key as JSON!");

    // Extract the public key from the JSON value
    let vuf_public_key = vuf_public_key_value
        .get(PUBLIC_KEY_FIELD_NAME)
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| {
            panic!(
                "VUF public key JSON does not contain the {} field!",
                PUBLIC_KEY_FIELD_NAME
            )
        });

    // Verify the public key matches the expected value
    if vuf_public_key != expected_vuf_public_key {
        panic!(
            "The VUF public key does not match the expected value! Actual: {:?}, Expected: {:?}",
            vuf_public_key, expected_vuf_public_key
        );
    }
}
