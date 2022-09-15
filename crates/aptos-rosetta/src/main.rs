// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::config::ApiConfig;
use aptos_node::AptosNodeArgs;
use aptos_rosetta::bootstrap;
use aptos_types::chain_id::ChainId;
use clap::Parser;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::time::Instant;

const REST_API_WAIT_DURATION_MS: u64 = 100;
const TOTAL_REST_API_WAIT_DURATION_S: u64 = 60;

#[tokio::main]
async fn main() {
    let args: CommandArgs = CommandArgs::parse();

    match args {
        CommandArgs::OnlineRemote(_) => {
            println!("aptos-rosetta: Starting Rosetta in Online remote (no local full node) mode")
        }
        CommandArgs::Online(_) => {
            println!("aptos-rosetta: Starting Rosetta in Online (with local full node) mode")
        }
        CommandArgs::Offline(_) => println!("aptos-rosetta: Starting Rosetta in Offline mode"),
    }

    // If we're in online mode, we run a full node side by side, the fullnode sets up the logger
    let _maybe_node = if let CommandArgs::Online(OnlineLocalArgs {
        ref node_args,
        ref online_args,
    }) = args
    {
        println!("aptos-rosetta: Starting local full node");
        let node_args = node_args.clone();
        let runtime = thread::spawn(move || node_args.run());

        // Wait and ensure the node is running on the URL
        let client = aptos_rest_client::Client::new(online_args.rest_api_url.clone());
        let mut successful = false;
        let total_wait_duration = Duration::from_secs(TOTAL_REST_API_WAIT_DURATION_S);
        let start = Instant::now();
        while start.elapsed() < total_wait_duration {
            if client.get_index_bcs().await.is_ok() {
                successful = true;
                break;
            }

            tokio::time::sleep(Duration::from_millis(REST_API_WAIT_DURATION_MS)).await;
        }

        // If it didn't start up, we need to crash
        if !successful {
            panic!(
                "aptos-rosetta: Local full node didn't start up on time after {} seconds at {}",
                TOTAL_REST_API_WAIT_DURATION_S, online_args.rest_api_url
            )
        }

        println!("aptos-rosetta: Local full node started successfully");
        Some(runtime)
    } else {
        // If we aren't running a full node, set up the logger now
        aptos_logger::Logger::new().init();
        None
    };

    println!("aptos-rosetta: Starting rosetta");
    // Ensure runtime for Rosetta is up and running
    let _rosetta = bootstrap(
        args.chain_id(),
        args.api_config(),
        args.rest_client(),
        args.synthetic_block_size(),
    )
    .expect("aptos-rosetta: Should bootstrap rosetta server");

    println!("aptos-rosetta: Rosetta started");
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

    /// Retrieve the block size
    fn synthetic_block_size(&self) -> Option<u16>;
}

/// Aptos Rosetta API Server
///
/// Provides an implementation of [Rosetta](https://www.rosetta-api.org/docs/Reference.html) on Aptos.
#[derive(Debug, Parser)]
#[clap(name = "aptos-rosetta", author, version, propagate_version = true)]
pub enum CommandArgs {
    /// Run a local online server that connects to a fullnode endpoint
    OnlineRemote(OnlineRemoteArgs),
    /// Run a local full node in tandem with Rosetta
    Online(OnlineLocalArgs),
    /// Run a local online server that doesn't connect to a fullnode endpoint
    Offline(OfflineArgs),
}

impl ServerArgs for CommandArgs {
    fn api_config(&self) -> ApiConfig {
        match self {
            CommandArgs::OnlineRemote(args) => args.api_config(),
            CommandArgs::Offline(args) => args.api_config(),
            CommandArgs::Online(args) => args.api_config(),
        }
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        match self {
            CommandArgs::OnlineRemote(args) => args.rest_client(),
            CommandArgs::Offline(args) => args.rest_client(),
            CommandArgs::Online(args) => args.rest_client(),
        }
    }

    fn chain_id(&self) -> ChainId {
        match self {
            CommandArgs::OnlineRemote(args) => args.chain_id(),
            CommandArgs::Offline(args) => args.chain_id(),
            CommandArgs::Online(args) => args.chain_id(),
        }
    }

    fn synthetic_block_size(&self) -> Option<u16> {
        match self {
            CommandArgs::OnlineRemote(args) => args.synthetic_block_size(),
            CommandArgs::Offline(args) => args.synthetic_block_size(),
            CommandArgs::Online(args) => args.synthetic_block_size(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct OfflineArgs {
    /// Listen address for the server. e.g. 0.0.0.0:8082
    #[clap(long, default_value = "0.0.0.0:8082")]
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
}

impl ServerArgs for OfflineArgs {
    fn api_config(&self) -> ApiConfig {
        ApiConfig {
            enabled: true,
            address: self.listen_address,
            tls_cert_path: self.tls_cert_path.clone(),
            tls_key_path: self.tls_key_path.clone(),
            content_length_limit: self.content_length_limit,
            ..Default::default()
        }
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        None
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    fn synthetic_block_size(&self) -> Option<u16> {
        None
    }
}

#[derive(Debug, Parser)]
pub struct OnlineRemoteArgs {
    #[clap(flatten)]
    offline_args: OfflineArgs,
    /// URL for the Aptos REST API. e.g. https://fullnode.devnet.aptoslabs.com
    #[clap(long, default_value = "http://localhost:8080")]
    rest_api_url: url::Url,

    /// In the event that block size is not provided, it will use the actual blocks
    /// if block size is provided, it will make synthetic blocks based on the transaction
    /// versions.  Each block will be `synthetic_block_size` transaction versions
    #[clap(long)]
    pub synthetic_block_size: Option<u16>,
}

impl ServerArgs for OnlineRemoteArgs {
    fn api_config(&self) -> ApiConfig {
        self.offline_args.api_config()
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        Some(aptos_rest_client::Client::new(self.rest_api_url.clone()))
    }

    fn chain_id(&self) -> ChainId {
        self.offline_args.chain_id
    }

    fn synthetic_block_size(&self) -> Option<u16> {
        self.synthetic_block_size
    }
}

#[derive(Debug, Parser)]
pub struct OnlineLocalArgs {
    #[clap(flatten)]
    online_args: OnlineRemoteArgs,
    #[clap(flatten)]
    node_args: AptosNodeArgs,
}

impl ServerArgs for OnlineLocalArgs {
    fn api_config(&self) -> ApiConfig {
        self.online_args.offline_args.api_config()
    }

    fn rest_client(&self) -> Option<aptos_rest_client::Client> {
        Some(aptos_rest_client::Client::new(
            self.online_args.rest_api_url.clone(),
        ))
    }

    fn chain_id(&self) -> ChainId {
        self.online_args.offline_args.chain_id
    }

    fn synthetic_block_size(&self) -> Option<u16> {
        self.online_args.synthetic_block_size
    }
}
