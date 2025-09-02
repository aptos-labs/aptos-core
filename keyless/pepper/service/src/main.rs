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
    vuf_keys::VUF_SK,
    watcher::start_external_resource_refresh_loop,
};
use aptos_types::keyless::test_utils::get_sample_iss;
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use std::{convert::Infallible, net::SocketAddr, ops::Deref, time::Duration};

#[tokio::main]
async fn main() {
    // Trigger private key loading.
    let _ = VUF_SK.deref();
    let _ = ACCOUNT_MANAGERS.deref();
    {
        let _db = ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await;
    }
    aptos_logger::Logger::new().init();
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

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(request_handler::handle_request))
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
