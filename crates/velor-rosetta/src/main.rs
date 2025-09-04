// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Runs the Rosetta server directly.

#![forbid(unsafe_code)]

use velor_config::config::{ApiConfig, DEFAULT_MAX_PAGE_SIZE};
use velor_logger::prelude::*;
use velor_node::VelorNodeArgs;
use velor_rosetta::{bootstrap, common::native_coin, types::Currency};
use velor_sdk::move_types::language_storage::StructTag;
use velor_types::chain_id::ChainId;
use clap::Parser;
use std::{
    collections::HashSet,
    fs::File,
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::time::Instant;

/// Poll every 100 ms
const DEFAULT_REST_API_WAIT_INTERVAL_MS: u64 = 100;
/// Log failures every 10 seconds
const LOG_INTERVAL_MS: u64 = 10_000;

#[tokio::main]
async fn main() {
    let args: CommandArgs = CommandArgs::parse();

    match args {
        CommandArgs::OnlineRemote(_) => {
            println!("velor-rosetta: Starting Rosetta in Online remote (no local full node) mode")
        },
        CommandArgs::Online(_) => {
            println!("velor-rosetta: Starting Rosetta in Online (with local full node) mode")
        },
        CommandArgs::Offline(_) => println!("velor-rosetta: Starting Rosetta in Offline mode"),
    }

    // If we're in online mode, we run a full node side by side, the fullnode sets up the logger
    let _maybe_node = if let CommandArgs::Online(OnlineLocalArgs {
        ref node_args,
        ref online_args,
    }) = args
    {
        println!("velor-rosetta: Starting local full node");
        let node_args = node_args.clone();
        let runtime = thread::spawn(move || node_args.run());

        // Wait and ensure the node is running on the URL
        let client = velor_rest_client::Client::new(online_args.rest_api_url.clone());
        let start = Instant::now();
        loop {
            match client.get_index_bcs().await {
                Ok(_) => {
                    break;
                },
                Err(err) => {
                    sample!(
                        SampleRate::Duration(Duration::from_millis(LOG_INTERVAL_MS)),
                        println!(
                            "velor-rosetta: Full node REST API isn't responding yet.  You should check the node logs.  It's been waiting {} seconds.  Error: {:?}",
                            start.elapsed().as_secs(),
                            err
                        )
                    );
                    tokio::time::sleep(Duration::from_millis(DEFAULT_REST_API_WAIT_INTERVAL_MS))
                        .await;
                },
            }
        }

        println!("velor-rosetta: Local full node started successfully");
        Some(runtime)
    } else {
        // If we aren't running a full node, set up the logger now
        velor_logger::Logger::new().init();
        None
    };

    println!("velor-rosetta: Starting rosetta");
    // Ensure runtime for Rosetta is up and running
    let _rosetta = bootstrap(
        args.chain_id(),
        args.api_config(),
        args.rest_client(),
        args.supported_currencies(),
    )
    .expect("velor-rosetta: Should bootstrap rosetta server");

    println!("velor-rosetta: Rosetta started");
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
    fn rest_client(&self) -> Option<velor_rest_client::Client>;

    /// Retrieve the chain id
    fn chain_id(&self) -> ChainId;

    /// Supported currencies for the service
    fn supported_currencies(&self) -> HashSet<Currency>;
}

/// Velor Rosetta API Server
///
/// Provides an implementation of [Rosetta](https://www.rosetta-api.org/docs/Reference.html) on Velor.
#[derive(Debug, Parser)]
#[clap(name = "velor-rosetta", author, version, propagate_version = true)]
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

    fn rest_client(&self) -> Option<velor_rest_client::Client> {
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

    fn supported_currencies(&self) -> HashSet<Currency> {
        match self {
            CommandArgs::OnlineRemote(args) => args.supported_currencies(),
            CommandArgs::Offline(args) => args.supported_currencies(),
            CommandArgs::Online(args) => args.supported_currencies(),
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
    #[clap(long, default_value_t = ChainId::test())]
    chain_id: ChainId,
    /// Page size for transactions APIs, must match the downstream node
    ///
    /// This can be configured to change performance characteristics
    #[clap(long, default_value_t = DEFAULT_MAX_PAGE_SIZE)]
    transactions_page_size: u16,

    /// A file of currencies to support other than APT
    ///
    /// Example file for testnet:
    /// ```json
    /// [
    ///   {
    ///     "symbol": "TC",
    ///     "decimals": 4,
    ///     "metadata": {
    ///       "fa_address": "0xb528ad40e472f8fcf0f21aa78aecd09fe68f6208036a5845e6d16b7d561c83b8",
    ///       "move_type": "0xf5a9b6ccc95f8ad3c671ddf1e227416e71f7bcd3c971efe83c0ae8e5e028350f::test_faucet::TestFaucetCoin"
    ///     }
    ///   },
    ///   {
    ///     "symbol": "TFA",
    ///     "decimals": 4,
    ///     "metadata": {
    ///       "fa_address": "0x7e51ad6e79cd113f5abe08f53ed6a3c2bfbf88561a24ae10b9e1e822e0623dfd"
    ///     }
    ///   }
    /// ]
    /// ```
    #[clap(long)]
    currency_config_file: Option<PathBuf>,
}

impl ServerArgs for OfflineArgs {
    fn api_config(&self) -> ApiConfig {
        ApiConfig {
            enabled: true,
            address: self.listen_address,
            tls_cert_path: self.tls_cert_path.clone(),
            tls_key_path: self.tls_key_path.clone(),
            content_length_limit: self.content_length_limit,
            max_transactions_page_size: self.transactions_page_size,
            ..Default::default()
        }
    }

    fn rest_client(&self) -> Option<velor_rest_client::Client> {
        None
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    fn supported_currencies(&self) -> HashSet<Currency> {
        let mut supported_currencies = HashSet::new();
        supported_currencies.insert(native_coin());

        if let Some(ref filepath) = self.currency_config_file {
            let file = File::open(filepath).unwrap();
            let currencies: Vec<Currency> = serde_json::from_reader(file).unwrap();
            for item in currencies.into_iter() {
                // Do a safety check on possible currencies on startup
                if item.symbol.as_str() == "" {
                    warn!(
                        "Currency {:?} has an empty symbol, and is being skipped",
                        item
                    );
                } else if let Some(metadata) = item.metadata.as_ref() {
                    if let Some(move_type) = metadata.move_type.as_ref() {
                        if StructTag::from_str(move_type).is_ok() {
                            supported_currencies.insert(item);
                            continue;
                        }
                    }
                    warn!(
                        "Currency {:?} has an invalid metadata coin type, and is being skipped",
                        item
                    );
                } else {
                    supported_currencies.insert(item);
                }
            }
        }

        supported_currencies
    }
}

#[derive(Debug, Parser)]
pub struct OnlineRemoteArgs {
    #[clap(flatten)]
    offline_args: OfflineArgs,
    /// URL for the Velor REST API. e.g. https://fullnode.devnet.velorlabs.com
    #[clap(long, default_value = "http://localhost:8080")]
    rest_api_url: url::Url,
    /// DEPRECATED: Owner addresses file as a YAML file with a list
    #[clap(long, value_parser)]
    owner_address_file: Option<PathBuf>,
}

impl ServerArgs for OnlineRemoteArgs {
    fn api_config(&self) -> ApiConfig {
        self.offline_args.api_config()
    }

    fn rest_client(&self) -> Option<velor_rest_client::Client> {
        Some(velor_rest_client::Client::new(self.rest_api_url.clone()))
    }

    fn chain_id(&self) -> ChainId {
        self.offline_args.chain_id
    }

    fn supported_currencies(&self) -> HashSet<Currency> {
        self.offline_args.supported_currencies()
    }
}

#[derive(Debug, Parser)]
pub struct OnlineLocalArgs {
    #[clap(flatten)]
    online_args: OnlineRemoteArgs,
    #[clap(flatten)]
    node_args: VelorNodeArgs,
}

impl ServerArgs for OnlineLocalArgs {
    fn api_config(&self) -> ApiConfig {
        self.online_args.offline_args.api_config()
    }

    fn rest_client(&self) -> Option<velor_rest_client::Client> {
        Some(velor_rest_client::Client::new(
            self.online_args.rest_api_url.clone(),
        ))
    }

    fn chain_id(&self) -> ChainId {
        self.online_args.offline_args.chain_id
    }

    fn supported_currencies(&self) -> HashSet<Currency> {
        self.online_args.offline_args.supported_currencies()
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    CommandArgs::command().debug_assert()
}
