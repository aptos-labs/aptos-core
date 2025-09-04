// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use velor::node::local_testnet::{
    faucet::FaucetManager,
    get_derived_test_dir,
    health_checker::HealthChecker,
    node::{build_node_config, NodeManager},
    traits::ServiceManager,
};
use velor_config::config::DEFAULT_GRPC_STREAM_PORT;
use velor_faucet_core::server::{FunderKeyEnum, RunConfig};
use rand::{rngs::StdRng, SeedableRng};
use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf};
use tokio::{
    fs::{create_dir_all, remove_dir_all},
    task::JoinSet,
};

const DEFAULT_SEED: [u8; 32] = [123; 32];
use url::Url;
/// Use a subfolder to store the indexer testing data, this is to avoid conflicts with localnet testing.
const INDEXER_TESTING_FOLDER: &str = "indexer-testing";
const FAUCET_DEFAULT_PORT: u16 = 8081;

/// ManagedNode is a managed node that can execute Move scripts and modules.
#[derive(Debug)]
pub struct ManagedNode {
    pub transaction_stream_url: Url,

    pub node: JoinSet<anyhow::Result<()>>,
}

impl ManagedNode {
    pub async fn start(
        node_config_path: Option<PathBuf>,
        node_data_dir: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
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
        let node = build_node_config(rng, &node_config_path, &None, false, node_dir.clone())
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
        let faucet_manager = create_faucet_manager(
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

        let transaction_stream_url = Url::parse("http://localhost:50051").unwrap();

        println!("\nTransaction generator is ready to execute.\n");
        Ok(Self {
            node: join_set,
            transaction_stream_url,
        })
    }

    /// Stops the node and the faucet.
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        println!("Stopping node service task...");
        self.node.abort_all();
        // The tasks spawned are cancelled; so the errors here(Err::Cancelled) are expected and ignored.
        while self.node.join_next().await.is_some() {
            println!("Node service task stopped.");
        }
        println!("====================");
        Ok(())
    }
}

fn create_faucet_manager(
    prerequisite_health_checkers: HashSet<HealthChecker>,
    faucet_port: u16,
    test_dir: PathBuf,
    node_api_url: Url,
) -> anyhow::Result<FaucetManager> {
    Ok(FaucetManager {
        config: RunConfig::build_for_cli(
            node_api_url.clone(),
            Ipv4Addr::LOCALHOST.to_string(),
            faucet_port,
            FunderKeyEnum::KeyFile(test_dir.join("mint.key")),
            true,
            None,
        ),
        prerequisite_health_checkers,
    })
}
