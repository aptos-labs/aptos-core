// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliError, CliResult, CliTypedResult};
use aptos_faucet::FaucetArgs;
use aptos_types::chain_id::ChainId;
use async_trait::async_trait;
use clap::Parser;
use hex::FromHex;
use rand::{rngs::StdRng, SeedableRng};
use reqwest::Url;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

/// CLI Tool for running a local testnet
///
#[derive(Parser)]
pub enum LocalTestnetTool {
    Node(RunTestNode),
}

impl LocalTestnetTool {
    pub async fn execute(self) -> CliResult {
        use LocalTestnetTool::*;
        match self {
            Node(tool) => tool.execute_serialized().await,
        }
    }
}

/// Run a single validator node testnet locally
#[derive(Parser)]
pub struct RunTestNode {
    /// An overridable config for the test node
    #[clap(long, parse(from_os_str))]
    config_path: Option<PathBuf>,
    /// The directory to save all files for the node
    #[clap(long, parse(from_os_str), default_value = ".aptos/local-testnet")]
    node_dir: PathBuf,
    /// Random seed for key generation in test mode
    #[clap(long, parse(try_from_str = FromHex::from_hex))]
    seed: Option<[u8; 32]>,
    /// Run a faucet alongside the node
    #[clap(long)]
    with_faucet: bool,
}

#[async_trait]
impl CliCommand<()> for RunTestNode {
    fn command_name(&self) -> &'static str {
        "RunTestNode"
    }

    async fn execute(mut self) -> CliTypedResult<()> {
        let rng = self
            .seed
            .map(StdRng::from_seed)
            .unwrap_or_else(StdRng::from_entropy);
        let node_dir = self.node_dir.clone();
        let config_path = self.config_path.clone();
        // Spawn the node in a separate thread
        let _node = thread::spawn(move || {
            aptos_node::load_test_environment(
                config_path,
                node_dir,
                false,
                false,
                cached_framework_packages::module_blobs().to_vec(),
                rng,
            )
        });

        // Run faucet if selected
        let _maybe_faucet = if self.with_faucet {
            // TODO: Handle if this port is changed
            let rest_url =
                Url::parse("http://localhost:8080").expect("Should be able to parse localhost");
            let rest_client = aptos_rest_client::Client::new(rest_url.clone());
            let interval = Duration::from_millis(500);
            let max_wait = Duration::from_secs(60);
            let start = Instant::now();
            let mut started_successfully = false;

            // Wait for the REST API to be ready
            while start.elapsed() < max_wait {
                if rest_client.get_index().await.is_ok() {
                    started_successfully = true;
                    break;
                }
                tokio::time::sleep(interval).await
            }

            if !started_successfully {
                return Err(CliError::UnexpectedError(
                    "Failed to startup local node before faucet".to_string(),
                ));
            }

            Some(
                FaucetArgs {
                    address: "127.0.0.1".to_string(),
                    port: 8081,
                    server_url: rest_url,
                    mint_key_file_path: self.node_dir.join("mint.key"),
                    mint_key: None,
                    mint_account_address: None,
                    chain_id: ChainId::test(),
                    maximum_amount: None,
                    do_not_delegate: false,
                }
                .run()
                .await,
            )
        } else {
            None
        };

        // Wait for an interrupt
        let term = Arc::new(AtomicBool::new(false));
        while !term.load(Ordering::Acquire) {
            std::thread::park();
        }
        Ok(())
    }
}
