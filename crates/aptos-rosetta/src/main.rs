// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use aptos_config::config::ApiConfig;
use aptos_rosetta::bootstrap;
use clap::Parser;

#[tokio::main]
async fn main() {
    aptos_logger::Logger::new().init();
    let args: RosettaServerArgs = RosettaServerArgs::parse();

    let rest_client = aptos_rest_client::Client::new(args.rest_api_url);
    let api_config = ApiConfig {
        enabled: true,
        address: args.listen_address,
        tls_cert_path: None,
        tls_key_path: None,
        content_length_limit: None
    };

    // Ensure runtime for Rosetta is up and running
    let _runtime = bootstrap(api_config, rest_client).expect("Should bootstrap");

    // Run until there is an interrupt
    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}

/// Aptos Rosetta API Server
///
/// Provides an implementation of [Rosetta](https://www.rosetta-api.org/docs/Reference.html) on Aptos.
#[derive(Debug, Parser)]
#[clap(name = "aptos-rosetta", author, version, propagate_version = true)]
struct RosettaServerArgs {
    /// Listen address for the server. e.g. 127.0.0.1:8080
    #[clap(long, default_value = "127.0.0.1:8080")]
    listen_address: SocketAddr,
    /// URL for the Aptos REST API. e.g. https://fullnode.devnet.aptoslabs.com
    #[clap(long, default_value = "https://fullnode.devnet.aptoslabs.com")]
    rest_api_url: url::Url,
}