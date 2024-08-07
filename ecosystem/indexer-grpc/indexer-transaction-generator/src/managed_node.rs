// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use aptos::node::local_testnet::{
    faucet::FaucetManager,
    get_derived_test_dir,
    node::{build_node_config, NodeManager},
    traits::ServiceManager,
};
use aptos_config::config::DEFAULT_GRPC_STREAM_PORT;
use clap::Parser;
use rand::{rngs::StdRng, SeedableRng};
use std::{net::Ipv4Addr, path::PathBuf};
use tokio::{
    fs::{create_dir_all, remove_dir_all},
    process::Child,
    task::JoinSet,
};

const DEFAULT_SEED: [u8; 32] = [123; 32];
use url::Url;
/// Use a subfolder to store the indexer testing data, this is to avoid conflicts with localnet testing.
const INDEXER_TESTING_FOLDER: &str = "indexer-testing";
const FAUCET_DEFAULT_PORT: u16 = 8081;

#[derive(Parser)]
pub struct LocalnetNodeArgs {
    /// Path to the node config file, `fullnode.yaml`.
    #[clap(long)]
    node_config_path: Option<PathBuf>,
    /// Path to the custom CLI binary to run a localnode.
    /// If this is set, the node will be started as a separate process.
    #[clap(long)]
    cli_binary_path: Option<PathBuf>,
    /// Path to the data directory for the node.
    #[clap(long)]
    node_data_dir: Option<PathBuf>,
}

impl LocalnetNodeArgs {
    pub async fn start_node(&self) -> anyhow::Result<ManagedNode> {
        ManagedNode::start(
            &self.node_config_path,
            self.cli_binary_path.clone(),
            self.node_data_dir.clone(),
        )
        .await
    }
}

/// Internal node type to manage the node lifecycle.
#[derive(Debug)]
enum LocalnetNodeType {
    // Node built from current source code.
    // It's managed under a JoinSet.
    BuiltIn(JoinSet<anyhow::Result<()>>),
    #[allow(dead_code)]
    // Custom CLI binary to run a localnode.
    // It's managed under a Child process.
    CustomCliBinary(Child),
}

/// ManagedNode is a managed node that can execute Move scripts and modules.
///   - BuiltIn: running in a different tokio task. If the transaction generation is done, abort the task and exit.
///   - CustomBinary: running in another process. If the transaction generation is done, the process is killed.
/// Both include a faucet service for funding accounts.
#[derive(Debug)]
pub struct ManagedNode {
    node: LocalnetNodeType,

    pub transaction_stream_url: Url,
}

impl ManagedNode {
    pub async fn start(
        node_config_path: &Option<PathBuf>,
        binary_path: Option<PathBuf>,
        node_data_dir: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let result = match binary_path {
            Some(_path) => {
                unimplemented!("Custom CLI binary is not supported yet");
            },
            None => {
                let node_dir = get_derived_test_dir(&node_data_dir)?.join(INDEXER_TESTING_FOLDER);
                // By default, we don't reuse the testnet folder.
                if node_dir.exists() {
                    remove_dir_all(node_dir.as_path()).await.context(format!(
                        "Failed to remove testnet folder at {:?}",
                        &node_dir
                    ))?;
                }
                create_dir_all(node_dir.as_path()).await.context(format!(
                    "Failed to create testnet folder at {:?}",
                    &node_dir
                ))?;
                let rng = StdRng::from_seed(DEFAULT_SEED);
                let node = build_node_config(rng, node_config_path, &None, false, node_dir.clone())
                    .context("Failed to build node config")?;

                let node_manager = NodeManager::new_with_config(
                    node,
                    Ipv4Addr::LOCALHOST,
                    node_dir.clone(),
                    true,
                    DEFAULT_GRPC_STREAM_PORT,
                    false,
                )
                .context("Failed to start node service manager")?;

                let node_health_checkers = node_manager.get_health_checkers();
                let faucet_manager = FaucetManager::new_for_indexer_testing(
                    node_health_checkers.clone(),
                    FAUCET_DEFAULT_PORT,
                    node_dir.clone(),
                    node_manager.get_node_api_url(),
                )
                .context("Failed to build faucet service manager")?;
                let faucet_health_checkers = faucet_manager.get_health_checkers();

                let managers: Vec<Box<dyn ServiceManager>> =
                    vec![Box::new(node_manager), Box::new(faucet_manager)];
                let mut join_set = JoinSet::new();
                for manager in managers {
                    join_set.spawn(manager.run());
                }

                let wait_for_startup_futures = faucet_health_checkers
                    .iter()
                    .map(|checker| checker.wait(None));
                for f in futures::future::join_all(wait_for_startup_futures).await {
                    f.context("Faucet service did not start up successfully")?;
                }
                LocalnetNodeType::BuiltIn(join_set)
            },
        };

        let transaction_stream_url = Url::parse("http://localhost:50051").unwrap();

        println!("\nTransaction generator is ready to execute.\n");
        Ok(Self {
            node: result,
            transaction_stream_url,
        })
    }

    /// Stops the node and the faucet.
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        match &mut self.node {
            LocalnetNodeType::BuiltIn(join_set) => {
                join_set.abort_all();
                while let Some(result) = join_set.join_next().await {
                    result
                        .context("ManagedNode JoinSet joining failure")?
                        .context("ManageNode task failed")?;
                }
            },
            LocalnetNodeType::CustomCliBinary(child) => {
                child.kill().await?;
            },
        }
        Ok(())
    }
}
