// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_service::{
    account_db::{init_account_db, ACCOUNT_RECOVERY_DB},
    account_managers::ACCOUNT_MANAGERS,
    groth16_vk::ONCHAIN_GROTH16_VK,
    jwk::{self, parse_jwks, DECODING_KEY_CACHE},
    keyless_config::ONCHAIN_KEYLESS_CONFIG,
    metrics::start_metric_server,
    request_handler,
    request_handler::DEFAULT_PEPPER_SERVICE_PORT,
    vuf_pub_key,
    watcher::start_external_resource_refresh_loop,
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
    aptos_logger::Logger::new().init();
    info!("Starting the Pepper service...");

    // Fetch the VUF public and private keypair (this will load the private key into memory)
    info!("Fetching the VUF public and private keypair for the pepper service...");
    let (vuf_public_key, vuf_private_key) = vuf_pub_key::get_pepper_service_vuf_keypair();
    info!("Retrieved the VUF public key: {:?}", vuf_public_key);

    // Trigger private key loading.
    let _ = ACCOUNT_MANAGERS.deref();
    {
        let _db = ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await;
    }
    start_metric_server();
    if let Ok(url) = std::env::var("ONCHAIN_GROTH16_VK_URL") {
        start_external_resource_refresh_loop(
            &url,
            Duration::from_secs(10),
            ONCHAIN_GROTH16_VK.clone(),
        );
    }
    if let Ok(url) = std::env::var("ONCHAIN_KEYLESS_CONFIG_URL") {
        start_external_resource_refresh_loop(
            &url,
            Duration::from_secs(10),
            ONCHAIN_KEYLESS_CONFIG.clone(),
        );
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

    let addr = SocketAddr::from(([0, 0, 0, 0], DEFAULT_PEPPER_SERVICE_PORT));

    // Wrap the VUF keypair in an Arc (to be shared across request handler threads)
    let vuf_key_pair = Arc::new((vuf_public_key, vuf_private_key));

    // Create the service function that handles the endpoint requests
    let make_service = make_service_fn(move |_conn| {
        let vuf_key_pair = vuf_key_pair.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                request_handler::handle_request(request, vuf_key_pair.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
