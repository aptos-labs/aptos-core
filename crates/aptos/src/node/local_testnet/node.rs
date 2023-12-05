// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{health_checker::HealthChecker, traits::ServiceManager, RunLocalTestnet};
use crate::node::local_testnet::utils::socket_addr_to_url;
use anyhow::{anyhow, Context, Result};
use aptos_config::config::{NodeConfig, DEFAULT_GRPC_STREAM_PORT};
use aptos_node::{load_node_config, start_test_environment_node};
use async_trait::async_trait;
use clap::Parser;
use maplit::hashset;
use rand::{rngs::StdRng, SeedableRng};
use reqwest::Url;
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    thread,
    time::Duration,
};

/// Args specific to running a node (and its components, e.g. the txn stream) in the
/// local testnet.
#[derive(Debug, Parser)]
pub struct NodeArgs {
    /// An overridable config template for the test node
    ///
    /// If provided, the config will be used, and any needed configuration for the local testnet
    /// will override the config's values
    #[clap(long, value_parser)]
    pub config_path: Option<PathBuf>,

    /// Path to node configuration file override for local test mode.
    ///
    /// If provided, the default node config will be overridden by the config in the given file.
    /// Cannot be used with --config-path
    #[clap(long, value_parser, conflicts_with("config_path"))]
    pub test_config_override: Option<PathBuf>,

    /// Random seed for key generation in test mode
    ///
    /// This allows you to have deterministic keys for testing
    #[clap(long, value_parser = aptos_node::load_seed)]
    pub seed: Option<[u8; 32]>,

    /// Do not run a transaction stream service alongside the node.
    ///
    /// Note: In reality this is not the same as running a Transaction Stream Service,
    /// it is just using the stream directly on the node, but in practice this
    /// distinction shouldn't matter.
    #[clap(long)]
    no_txn_stream: bool,

    /// The port at which to expose the grpc transaction stream.
    #[clap(long, default_value_t = DEFAULT_GRPC_STREAM_PORT)]
    txn_stream_port: u16,
}

#[derive(Clone, Debug)]
pub struct NodeManager {
    config: NodeConfig,
    test_dir: PathBuf,
}

impl NodeManager {
    pub fn new(args: &RunLocalTestnet, bind_to: Ipv4Addr, test_dir: PathBuf) -> Result<Self> {
        let rng = args
            .node_args
            .seed
            .map(StdRng::from_seed)
            .unwrap_or_else(StdRng::from_entropy);

        // If there is a config on disk, this function will use that. If not, it will
        // create a new one, taking the config_path and test_config_override arguments
        // into account.
        let mut node_config = load_node_config(
            &args.node_args.config_path,
            &args.node_args.test_config_override,
            &test_dir,
            false,
            false,
            aptos_cached_packages::head_release_bundle(),
            rng,
        )
        .context("Failed to load / create config for node")?;

        eprintln!();

        // Enable the grpc stream on the node if we will run a txn stream service.
        let run_txn_stream = !args.node_args.no_txn_stream;
        node_config.indexer_grpc.enabled = run_txn_stream;
        node_config.indexer_grpc.use_data_service_interface = run_txn_stream;
        node_config
            .indexer_grpc
            .address
            .set_port(args.node_args.txn_stream_port);

        // So long as the indexer relies on storage indexing tables, this must be set
        // for the indexer GRPC stream on the node to work.
        node_config.storage.enable_indexer = run_txn_stream;

        // Bind to the requested address.
        node_config.api.address.set_ip(IpAddr::V4(bind_to));
        node_config.indexer_grpc.address.set_ip(IpAddr::V4(bind_to));
        node_config.admin_service.address = bind_to.to_string();
        node_config.inspection_service.address = bind_to.to_string();

        Ok(NodeManager {
            config: node_config,
            test_dir,
        })
    }

    pub fn get_node_api_url(&self) -> Url {
        socket_addr_to_url(&self.config.api.address, "http").unwrap()
    }

    pub fn get_data_service_url(&self) -> Url {
        socket_addr_to_url(&self.config.indexer_grpc.address, "http").unwrap()
    }
}

#[async_trait]
impl ServiceManager for NodeManager {
    fn get_name(&self) -> String {
        "Node API".to_string()
    }

    /// We return health checkers for both the Node API and the txn stream (if enabled).
    /// As it is now, it is fine to make downstream services wait for both but if that
    /// changes we can refactor.
    fn get_health_checkers(&self) -> HashSet<HealthChecker> {
        let node_api_url = self.get_node_api_url();
        let mut checkers = HashSet::new();
        checkers.insert(HealthChecker::NodeApi(node_api_url));
        if self.config.indexer_grpc.enabled {
            let data_service_url =
                socket_addr_to_url(&self.config.indexer_grpc.address, "http").unwrap();
            checkers.insert(HealthChecker::DataServiceGrpc(data_service_url));
        }
        checkers
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        // The node doesn't depend on anything, we start it first.
        hashset! {}
    }

    /// Spawn the node on a thread and then create a future that just waits for it to
    /// exit (which should never happen) forever. This is necessary because there is
    /// no async function we can use to run the node.
    async fn run_service(self: Box<Self>) -> Result<()> {
        let node_thread_handle = thread::spawn(move || {
            let result = start_test_environment_node(self.config, self.test_dir, false);
            eprintln!("Node stopped unexpectedly {:#?}", result);
        });

        // This just waits for the node thread forever.
        loop {
            if node_thread_handle.is_finished() {
                return Err(anyhow!("Node thread finished unexpectedly"));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
