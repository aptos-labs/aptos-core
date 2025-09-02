// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_service::{
    account_db::{init_account_db, ACCOUNT_RECOVERY_DB},
    account_managers::ACCOUNT_MANAGERS,
    cached_resources,
    cached_resources::CachedResources,
    jwk::{self, parse_jwks, DECODING_KEY_CACHE},
    metrics,
    metrics::DEFAULT_METRICS_SERVER_PORT,
    request_handler,
    request_handler::DEFAULT_PEPPER_SERVICE_PORT,
    vuf_pub_key,
};
use aptos_logger::info;
use aptos_types::keyless::test_utils::get_sample_iss;
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use std::{convert::Infallible, net::SocketAddr, ops::Deref, sync::Arc, time::Duration};

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
    let cached_resources = cached_resources::start_cached_resource_fetcher();

    // Trigger private key loading.
    let _ = ACCOUNT_MANAGERS.deref();
    {
        let _db = ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await;
    }

    // TODO: JWKs should be from on-chain states?
    jwk::start_jwk_refresh_loop(
        "https://accounts.google.com",
        "https://www.googleapis.com/oauth2/v3/certs",
        Duration::from_secs(10),
    );
    jwk::start_jwk_refresh_loop(
        "https://appleid.apple.com",
        "https://appleid.apple.com/auth/keys",
        Duration::from_secs(10),
    );

    let test_jwk = include_str!("../../../../types/src/jwks/rsa/secure_test_jwk.json");
    DECODING_KEY_CACHE.insert(
        get_sample_iss(),
        parse_jwks(test_jwk).expect("test jwk should parse"),
    );

    // Start the pepper service
    let vuf_keypair = Arc::new((vuf_public_key, vuf_private_key));
    start_pepper_service(vuf_keypair, cached_resources).await;
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
    cached_resources: CachedResources,
) {
    info!("Starting the Pepper service request handler...");

    // Create the service function that handles the endpoint requests
    let make_service = make_service_fn(move |_conn| {
        let vuf_keypair = vuf_keypair.clone();
        let cached_resources = cached_resources.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                request_handler::handle_request(
                    request,
                    vuf_keypair.clone(),
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
