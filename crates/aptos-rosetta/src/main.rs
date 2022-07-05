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
    let args: CommandArgs = CommandArgs::parse();

    // Ensure runtime for Rosetta is up and running
    let _runtime = bootstrap(
        args.block_size(),
        args.chain_id(),
        args.api_config(),
        args.rest_client(),
    )
    .expect("Should bootstrap");

    // Run until there is an interrupt
    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}

/// A trait to provide common values from both online and offline mode
trait ServerArgs {
    /// Retrieve the API config for the local server
    fn api_config(&self) -> ApiConfig;

    /// Retrieve the optional rest client for the local server
    fn rest_client(&self) -> Option<aptos_rest_client::Client>;

    /// Retrieve the chain id
    fn chain_id(&self) -> ChainId;

    fn block_size(&self) -> u64;
}

/// Aptos Rosetta API Server
///
/// Provides an implementation of [Rosetta](https://www.rosetta-api.org/docs/Reference.html) on Aptos.
#[derive(Debug, Parser)]
#[clap(name = "aptos-rosetta", author, version, propagate_version = true)]
pub enum CommandArgs {
    /// Run a local online server that connects to a fullnode endpoint
    Online(OnlineArgs),
    /// Run a local online server that doesn't connect to a fullnode endpoint
    Offline(OfflineArgs),
}

impl ServerArgs for CommandArgs {
    fn api_config(&self) -> ApiConfig {
        match self {
            CommandArgs::Online(args) => args.api_config(),
            CommandArgs::Offline(args) => args.api_config(),
        }
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        match self {
            CommandArgs::Online(args) => args.rest_client(),
            CommandArgs::Offline(args) => args.rest_client(),
        }
    }

    fn chain_id(&self) -> ChainId {
        match self {
            CommandArgs::Online(args) => args.chain_id(),
            CommandArgs::Offline(args) => args.chain_id(),
        }
    }

    fn block_size(&self) -> u64 {
        match self {
            CommandArgs::Online(args) => args.block_size(),
            CommandArgs::Offline(args) => args.block_size(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct OfflineArgs {
    /// Listen address for the server. e.g. 127.0.0.1:8080
    #[clap(long, default_value = "127.0.0.1:8082")]
    listen_address: SocketAddr,
    /// Path to TLS cert for HTTPS support
    #[clap(long)]
    tls_cert_path: Option<String>,
    /// Path to TLS key for HTTPS support
    #[clap(long)]
    tls_key_path: Option<String>,
    /// Limit to content length on all requests
    #[clap(long)]
    content_length_limit: Option<u64>,
    /// ChainId to be used for the server e.g. TESTNET
    #[clap(long, default_value = "TESTING")]
    chain_id: ChainId,
    /// Block size to emulate blocks
    #[clap(long, default_value_t = 1000)]
    block_size: u64,
}

impl ServerArgs for OfflineArgs {
    fn api_config(&self) -> ApiConfig {
        ApiConfig {
            enabled: true,
            address: self.listen_address,
            tls_cert_path: self.tls_cert_path.clone(),
            tls_key_path: self.tls_key_path.clone(),
            content_length_limit: self.content_length_limit,
        }
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        None
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    fn block_size(&self) -> u64 {
        self.block_size
    }
}

#[derive(Debug, Parser)]
pub struct OnlineArgs {
    #[clap(flatten)]
    offline_args: OfflineArgs,
    /// URL for the Aptos REST API. e.g. https://fullnode.devnet.aptoslabs.com
    #[clap(long, default_value = "https://fullnode.devnet.aptoslabs.com")]
    rest_api_url: url::Url,
}

impl ServerArgs for OnlineArgs {
    fn api_config(&self) -> ApiConfig {
        self.offline_args.api_config()
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        Some(aptos_rest_client::Client::new(self.rest_api_url.clone()))
    }

    fn chain_id(&self) -> ChainId {
        self.offline_args.chain_id
    }

    fn block_size(&self) -> u64 {
        self.offline_args.block_size
    }
}
