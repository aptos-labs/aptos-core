// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{health_checker::HealthChecker, traits::ServiceManager, RunLocalnet};
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
/// localnet.
#[derive(Debug, Parser)]
pub struct NodeArgs {
    /// An overridable config template for the test node
    ///
    /// If provided, the config will be used, and any needed configuration for the localnet
    /// will override the config's values
    #[clap(long, value_parser)]
    pub config_path: Option<PathBuf>,

    /// Path to node configuration file override for local test mode.
    ///
    /// If provided, the default node config will be overridden by the config in the given file.
    /// Cannot be used with --config-path
    #[clap(long, value_parser, conflicts_with("config_path"))]
    pub test_config_override: Option<PathBuf>,

    /// Optimize the node for higher performance.
    ///
    /// Note: This is only useful for e2e performance testing, and should not be used in production.
    #[clap(long)]
    pub performance: bool,

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

    /// If set we won't run the node at all.
    //
    // Note: I decided that since running multiple partial localnets is a rare
    // case that only core devs would ever really want, it wasn't worth making the code
    // much more complex to support that case "first class". Instead, we have this flag
    // that does everything else to set up running the node, but never actually runs
    // it. This is useful if you want to invoke the CLI once to run a node + txn stream
    // and invoke it again to run processors + indexer API. You might want to do this
    // for compatibility testing. If you use this flag and there _isn't_ a node already
    // running at the expected port, the processors will fail to connect to the txn
    // stream (since there isn't one) and the localnet will crash.
    //
    // If we do change our minds on this one day, the correct way to do this would be
    // to let the user instead pass in a bunch of flags that declare where an existing
    // node is running, separate service configs from their manager, return a config
    // instead of a manager, etc.
    //
    // Because this flag is a bit of a footgun we hide it from regular users.
    #[clap(long, hide = true)]
    pub no_node: bool,
}

#[derive(Clone, Debug)]
pub struct NodeManager {
    config: NodeConfig,
    test_dir: PathBuf,
    no_node: bool,
}

pub fn build_node_config(
    rng: StdRng,
    config_path: &Option<PathBuf>,
    test_config_override: &Option<PathBuf>,
    performance: bool,
    test_dir: PathBuf,
) -> Result<NodeConfig> {
    // If there is a config on disk, this function will use that. If not, it will
    // create a new one, taking the config_path and test_config_override arguments
    // into account.
    load_node_config(
        config_path,
        test_config_override,
        &test_dir,
        false,
        false,
        performance,
        aptos_cached_packages::head_release_bundle(),
        rng,
    )
    .context("Failed to load / create config for node")
}

impl NodeManager {
    pub fn new(args: &RunLocalnet, bind_to: Ipv4Addr, test_dir: PathBuf) -> Result<Self> {
        let rng = args
            .node_args
            .seed
            .map(StdRng::from_seed)
            .unwrap_or_else(StdRng::from_entropy);

        let node_config = build_node_config(
            rng,
            &args.node_args.config_path,
            &args.node_args.test_config_override,
            args.node_args.performance,
            test_dir.clone(),
        )?;
        Self::new_with_config(
            node_config,
            bind_to,
            test_dir,
            !args.node_args.no_txn_stream,
            args.node_args.txn_stream_port,
            args.node_args.no_node,
        )
    }

    pub fn new_with_config(
        mut node_config: NodeConfig,
        bind_to: Ipv4Addr,
        test_dir: PathBuf,
        run_txn_stream: bool,
        txn_stream_port: u16,
        no_node: bool,
    ) -> Result<Self> {
        eprintln!();

        // Enable the grpc stream on the node if we will run a txn stream service.
        node_config.indexer_grpc.enabled = run_txn_stream;
        node_config.indexer_grpc.use_data_service_interface = run_txn_stream;
        node_config.indexer_grpc.address.set_port(txn_stream_port);

        node_config.indexer_table_info.table_info_service_mode = match run_txn_stream {
            // Localnet should be responsible for backup or restore of table info tables.
            true => aptos_config::config::TableInfoServiceMode::IndexingOnly,
            false => aptos_config::config::TableInfoServiceMode::Disabled,
        };

        // Bind to the requested address.
        node_config.api.address.set_ip(IpAddr::V4(bind_to));
        node_config.indexer_grpc.address.set_ip(IpAddr::V4(bind_to));
        node_config.admin_service.address = bind_to.to_string();
        node_config.inspection_service.address = bind_to.to_string();
        node_config.indexer_db_config.enable_event = true;
        node_config.indexer_db_config.enable_statekeys = true;
        node_config.indexer_db_config.enable_transaction = true;

        Ok(NodeManager {
            config: node_config,
            test_dir,
            no_node,
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
        // Don't actually run the node, just idle.
        if self.no_node {
            loop {
                tokio::time::sleep(Duration::from_millis(10000)).await;
            }
        }

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
