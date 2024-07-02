// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use aptos::node::local_testnet::{
    faucet::FaucetManager, get_derived_test_dir, node::NodeManager, traits::ServiceManager,
};
use std::path::PathBuf;
use tokio::{
    fs::{create_dir_all, remove_dir_all},
    process::Child,
    task::JoinSet,
};
/// Internal node type to manage the node lifecycle.
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
///   - BuiltIn: running in a different thread. If the transaction generation is done, abort the thread and exit.
///   - CustomBinary: running in another process. If the transaction generation is done, the process is killed.
/// Both include a faucet service for funding accounts.
pub struct ManagedNode {
    node: LocalnetNodeType,
}

impl ManagedNode {
    pub async fn start(
        node_config_path: &Option<PathBuf>,
        binary_path: Option<PathBuf>,
        input_test_dir: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let result = match binary_path {
            Some(_path) => {
                unimplemented!("Custom binary node is not supported yet");
            },
            None => {
                let test_dir = get_derived_test_dir(&input_test_dir)?;
                // By default, we don't reuse the testnet folder.
                if test_dir.exists() {
                    remove_dir_all(test_dir.as_path()).await.context(format!(
                        "Failed to remove testnet folder at {:?}",
                        &test_dir
                    ))?;
                }
                create_dir_all(test_dir.as_path()).await.context(format!(
                    "Failed to create testnet folder at {:?}",
                    &test_dir
                ))?;
                let node_manager =
                    NodeManager::new_for_indexer_testing(node_config_path, test_dir.clone())
                        .context("Failed to start node service manager")?;
                let node_health_checkers = node_manager.get_health_checkers();
                let faucet_manager = FaucetManager::new_for_indexer_testing(
                    node_health_checkers.clone(),
                    8081,
                    test_dir.clone(),
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
        println!("\nTransaction generator is ready to execute.\n");
        Ok(Self { node: result })
    }

    /// Stops the node and the faucet.
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        match &mut self.node {
            LocalnetNodeType::BuiltIn(join_set) => {
                join_set.abort_all();
            },
            LocalnetNodeType::CustomCliBinary(child) => {
                child.kill().await?;
            },
        }
        Ok(())
    }
}
