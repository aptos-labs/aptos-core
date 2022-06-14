// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::config::ApiConfig;
use aptos_rosetta::bootstrap;
use aptos_types::chain_id::ChainId;
use clap::Parser;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[tokio::main]
async fn main() {
    aptos_logger::Logger::new().init();
    let args: RosettaServerArgs = RosettaServerArgs::parse();

    let rest_client = aptos_rest_client::Client::new(args.rest_api_url);
    let api_config = ApiConfig {
        enabled: true,
        address: args.listen_address,
        tls_cert_path: args.tls_cert_path,
        tls_key_path: args.tls_key_path,
        content_length_limit: args.content_length_limit,
    };

    // Ensure runtime for Rosetta is up and running
    let _runtime = bootstrap(args.chain_id, api_config, rest_client).expect("Should bootstrap");

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
pub struct RosettaServerArgs {
    /// Listen address for the server. e.g. 127.0.0.1:8080
    #[clap(long, default_value = "127.0.0.1:8080")]
    listen_address: SocketAddr,
    /// URL for the Aptos REST API. e.g. https://fullnode.devnet.aptoslabs.com
    #[clap(long, default_value = "https://fullnode.devnet.aptoslabs.com")]
    rest_api_url: url::Url,
    /// ChainId to be used for the server e.g. TESTNET
    #[clap(long, default_value = "TESTING")]
    chain_id: ChainId,
    /// Path to TLS cert for HTTPS support
    #[clap(long)]
    tls_cert_path: Option<String>,
    /// Path to TLS key for HTTPS support
    #[clap(long)]
    tls_key_path: Option<String>,
    /// Limit to content length on all requests
    #[clap(long)]
    content_length_limit: Option<u64>,
}
