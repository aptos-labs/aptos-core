// Copyright © Aptos Foundation

use aptos_crypto::ed25519::Ed25519PublicKey;
use axum::{
    routing::{get, post},
    Router,
};
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use http::Method;
use prover_service::{
    config::*,
    *,
    prover_key::cached_prover_key
};
use rust_rapidsnark::FullProver;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any)
        // allow cross-origin requests
        .allow_headers(Any);

    // init tracing
    logging::init_tracing().expect("Couldn't init tracing.");

    // read config and secret key
    let ProverServerConfig {
        zkey_path,
        witness_gen_binary_folder_path,
        test_verification_key_path: _,
        oidc_providers,
        jwk_refresh_rate_secs,
        port,
        metrics_port: _,
    } = Figment::new()
        .merge(Yaml::file(config::CONFIG_FILE_PATH))
        .merge(Env::raw())
        .extract()
        .expect("Couldn't load config");

    let ProverServerSecrets { private_key } = Figment::new()
        .merge(Env::raw())
        .extract()
        .expect("Couldn't load private key from environment variable PRIVATE_KEY");

    let zkey_path = cached_prover_key().await;

    // init state
    let public_key: Ed25519PublicKey = (&private_key).into();
    let full_prover = FullProver::new(&zkey_path, &witness_gen_binary_folder_path)
        .expect("failed to initialize rapidsnark prover");
    let metrics = metrics::ProverServerMetrics::new();
    let state = Arc::new(Mutex::new(ProverServerState {
        full_prover,
        public_key,
        private_key,
        metrics,
    }));

    jwk_fetching::init_jwk_fetching(&oidc_providers, Duration::from_secs(jwk_refresh_rate_secs))
        .await;

    // init axum and serve
    let app = Router::new()
        .route("/v0/prove", post(handlers::prove_handler))
        .route("/metrics", get(handlers::metrics_handler))
        .route("/healthcheck", get(handlers::healthcheck_handler))
        .fallback(handlers::fallback_handler)
        .with_state(state)
        .layer(ServiceBuilder::new().layer(cors));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
