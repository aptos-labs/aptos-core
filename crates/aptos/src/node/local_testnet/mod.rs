// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod health_checker;
mod logging;
mod ready_server;
mod utils;

use self::{
    health_checker::HealthChecker,
    logging::ThreadNameMakeWriter,
    ready_server::{run_ready_server, ReadyServerConfig},
    utils::socket_addr_to_url,
};
use crate::{
    common::{
        types::{CliCommand, CliError, CliTypedResult, ConfigSearchMode, PromptOptions},
        utils::prompt_yes_with_override,
    },
    config::GlobalConfig,
};
use anyhow::Context;
use aptos_config::config::{NodeConfig, DEFAULT_GRPC_STREAM_PORT};
use aptos_faucet_core::server::{FunderKeyEnum, RunConfig as FaucetConfig};
use aptos_indexer_grpc_server_framework::setup_logging;
use aptos_logger::debug;
use aptos_node::{load_node_config, start_test_environment_node};
use async_trait::async_trait;
use clap::Parser;
use futures::{Future, FutureExt};
use rand::{rngs::StdRng, SeedableRng};
use reqwest::Url;
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    pin::Pin,
    thread,
    time::Duration,
};
use tokio::task::JoinHandle;
use tracing_subscriber::fmt::MakeWriter;

const TESTNET_FOLDER: &str = "testnet";

/// Run a local testnet
///
/// This local testnet will run it's own genesis and run as a single node network
/// locally. A faucet and grpc transaction stream will run alongside the node unless
/// you specify otherwise with --no-faucet and --no-txn-stream respectively.
#[derive(Parser)]
pub struct RunLocalTestnet {
    /// An overridable config template for the test node
    ///
    /// If provided, the config will be used, and any needed configuration for the local testnet
    /// will override the config's values
    #[clap(long, value_parser)]
    config_path: Option<PathBuf>,

    /// The directory to save all files for the node
    ///
    /// Defaults to .aptos/testnet
    #[clap(long, value_parser)]
    test_dir: Option<PathBuf>,

    /// Path to node configuration file override for local test mode.
    ///
    /// If provided, the default node config will be overridden by the config in the given file.
    /// Cannot be used with --config-path
    #[clap(long, value_parser, conflicts_with("config_path"))]
    test_config_override: Option<PathBuf>,

    /// Random seed for key generation in test mode
    ///
    /// This allows you to have deterministic keys for testing
    #[clap(long, value_parser = aptos_node::load_seed)]
    seed: Option<[u8; 32]>,

    /// Clean the state and start with a new chain at genesis
    ///
    /// This will wipe the aptosdb in `test-dir` to remove any incompatible changes, and start
    /// the chain fresh.  Note, that you will need to publish the module again and distribute funds
    /// from the faucet accordingly
    #[clap(long)]
    force_restart: bool,

    /// Port to run the faucet on.
    ///
    /// When running, you'll be able to use the faucet at `http://127.0.0.1:<port>/mint` e.g.
    /// `http//127.0.0.1:8081/mint`
    #[clap(long, default_value_t = 8081)]
    faucet_port: u16,

    /// Do not run a faucet alongside the node.
    ///
    /// Running a faucet alongside the node allows you to create and fund accounts
    /// for testing.
    #[clap(long)]
    no_faucet: bool,

    /// This does nothing, we already run a faucet by default. We only keep this here
    /// for backwards compatibility with tests. We will remove this once the commit
    /// that added --no-faucet makes its way to the testnet branch.
    #[clap(long, hide = true)]
    with_faucet: bool,

    /// Disable the delegation of faucet minting to a dedicated account.
    #[clap(long)]
    do_not_delegate: bool,

    /// Do not run a transaction stream service alongside the node.
    ///
    /// Note: In reality this is not the same as running a Transaction Stream Service,
    /// it is just using the stream from the node, but in practice this distinction
    /// shouldn't matter.
    #[clap(long)]
    no_txn_stream: bool,

    /// The port at which to expose the grpc transaction stream.
    #[clap(long, default_value_t = DEFAULT_GRPC_STREAM_PORT)]
    txn_stream_port: u16,

    #[clap(flatten)]
    ready_server_config: ReadyServerConfig,

    #[clap(flatten)]
    prompt_options: PromptOptions,
}

#[derive(Debug)]
struct AllConfigs {
    ready_server_config: ReadyServerConfig,
    node_config: NodeConfig,
    faucet_config: Option<FaucetConfig>,
}

impl AllConfigs {
    pub fn get_node_api_url(&self) -> Url {
        socket_addr_to_url(&self.node_config.api.address, "http").unwrap()
    }
}

impl RunLocalTestnet {
    /// This function builds all the configs we need to run each of the requested
    /// services. We separate creating configs and spawning services to keep the
    /// code clean. This could also allow us to one day have two phases for starting
    /// a local testnet, in which you can alter the configs on disk between each phase.
    fn build_configs(&self, test_dir: PathBuf) -> anyhow::Result<AllConfigs> {
        let rng = self
            .seed
            .map(StdRng::from_seed)
            .unwrap_or_else(StdRng::from_entropy);

        // If there is a config on disk, this function will use that. If not, it will
        // create a new one, taking the config_path and test_config_override arguments
        // into account.
        let mut node_config = load_node_config(
            &self.config_path,
            &self.test_config_override,
            &test_dir,
            false,
            false,
            aptos_cached_packages::head_release_bundle(),
            rng,
        )
        .context("Failed to load / create config for node")?;

        eprintln!();

        // Enable the grpc stream on the node if we will run a txn stream service.
        let run_txn_stream = !self.no_txn_stream;
        node_config.indexer_grpc.enabled = run_txn_stream;
        node_config.indexer_grpc.use_data_service_interface = run_txn_stream;
        node_config
            .indexer_grpc
            .address
            .set_port(self.txn_stream_port);

        // So long as the indexer relies on storage indexing tables, this must be set
        // for the indexer GRPC stream on the node to work.
        node_config.storage.enable_indexer = run_txn_stream;

        let node_api_url = socket_addr_to_url(&node_config.api.address, "http").unwrap();

        let faucet_config = if self.no_faucet {
            None
        } else {
            Some(FaucetConfig::build_for_cli(
                node_api_url.clone(),
                self.faucet_port,
                FunderKeyEnum::KeyFile(test_dir.join("mint.key")),
                self.do_not_delegate,
                None,
            ))
        };

        Ok(AllConfigs {
            ready_server_config: self.ready_server_config.clone(),
            node_config,
            faucet_config,
        })
    }

    // Note: These start_* functions (e.g. start_node) can run checks prior to
    // returning the future for the service, for example to ensure that a prerequisite
    // service has started. They cannot however do anything afterwards. For that,
    // you probably want to define a HealthCheck to register with wait_for_startup.

    /// Spawn the node on a thread and then create a future that just waits for it to
    /// exit (which should never happen) forever. This is necessary because there is
    /// no async function we can use to run the node.
    async fn start_node(
        &self,
        test_dir: PathBuf,
        config: NodeConfig,
    ) -> CliTypedResult<impl Future<Output = ()>> {
        let node_thread_handle = thread::spawn(move || {
            let result = start_test_environment_node(config, test_dir, false);
            eprintln!("Node stopped unexpectedly {:#?}", result);
        });

        // This just waits for the node thread forever.
        let node_future = async move {
            loop {
                if node_thread_handle.is_finished() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        };

        Ok(node_future)
    }

    /// Run the faucet.
    async fn start_faucet(
        &self,
        config: FaucetConfig,
        node_api_url: Url,
    ) -> CliTypedResult<impl Future<Output = ()>> {
        HealthChecker::NodeApi(node_api_url)
            .wait(Some("Faucet"))
            .await?;

        // Start the faucet
        Ok(config.run().map(|result| {
            eprintln!("Faucet stopped unexpectedly {:#?}", result);
        }))
    }

    /// Run the ready server.
    async fn start_ready_server(
        &self,
        health_checks: Vec<HealthChecker>,
    ) -> CliTypedResult<impl Future<Output = ()>> {
        let config = self.ready_server_config.clone();
        Ok(run_ready_server(health_checks, config).map(|result| {
            eprintln!("Faucet stopped unexpectedly {:#?}", result);
        }))
    }

    /// Wait for many services to start up. This prints a message like "X is starting,
    /// please wait..." for each service and then "X is running. Endpoint: <url>"
    /// when it's ready.
    async fn wait_for_startup<'a>(&self, health_checks: &Vec<HealthChecker>) -> CliTypedResult<()> {
        let mut futures: Vec<Pin<Box<dyn futures::Future<Output = anyhow::Result<()>> + Send>>> =
            Vec::new();

        for health_check in health_checks {
            eprintln!("{} is starting, please wait...", health_check);
            let fut = async move {
                health_check.wait(None).await?;
                eprintln!(
                    "{} is running. Endpoint: {}",
                    health_check,
                    health_check.address_str()
                );
                Ok(())
            };
            futures.push(Box::pin(fut));
        }

        eprintln!();

        // We use join_all because we expect all of these to return.
        for f in futures::future::join_all(futures).await {
            f.map_err(|err| {
                CliError::UnexpectedError(format!(
                    "One of the services failed to start up: {:?}",
                    err
                ))
            })?;
        }

        eprintln!("\nAll services are running, you can now use the local testnet!");

        Ok(())
    }
}

#[async_trait]
impl CliCommand<()> for RunLocalTestnet {
    fn command_name(&self) -> &'static str {
        "RunLocalTestnet"
    }

    async fn execute(mut self) -> CliTypedResult<()> {
        let global_config = GlobalConfig::load().context("Failed to load global config")?;
        let test_dir = match &self.test_dir {
            Some(test_dir) => test_dir.clone(),
            None => global_config
                .get_config_location(ConfigSearchMode::CurrentDirAndParents)?
                .join(TESTNET_FOLDER),
        };

        // If asked, remove the current test directory and start with a new node.
        if test_dir.exists() && self.force_restart {
            prompt_yes_with_override(
                "Are you sure you want to delete the existing local testnet data?",
                self.prompt_options,
            )?;
            remove_dir_all(test_dir.as_path()).map_err(|err| {
                CliError::IO(format!("Failed to delete {}", test_dir.display()), err)
            })?;
        }

        if !test_dir.exists() {
            debug!("Test directory does not exist, creating it: {:?}", test_dir);
            create_dir_all(test_dir.as_path()).map_err(|err| {
                CliError::IO(format!("Failed to create {}", test_dir.display()), err)
            })?;
            debug!("Created test directory: {:?}", test_dir);
        }

        // Set up logging for anything that uses tracing. These logs will go to
        // different directories based on the name of the runtime.
        let td = test_dir.clone();
        let make_writer =
            move || ThreadNameMakeWriter::new(td.clone()).make_writer() as Box<dyn std::io::Write>;
        setup_logging(Some(Box::new(make_writer)));

        let all_configs = self
            .build_configs(test_dir.clone())
            .context("Failed to build configs")?;

        let node_api_url = all_configs.get_node_api_url();

        let AllConfigs {
            ready_server_config,
            node_config,
            faucet_config,
        } = all_configs;

        // Collect all the health checks we want to run.
        let mut health_checks = Vec::new();
        health_checks.push(HealthChecker::NodeApi(node_api_url.clone()));

        if let Some(config) = &faucet_config {
            let url = Url::parse(&format!(
                "http://{}:{}",
                config.server_config.listen_address, config.server_config.listen_port
            ))
            .unwrap();
            health_checks.push(HealthChecker::Http(url, "Faucet"));
        }

        if !self.no_txn_stream {
            let url = socket_addr_to_url(&node_config.indexer_grpc.address, "http")?;
            health_checks.push(HealthChecker::DataServiceGrpc(url));
        }

        // Build tasks for each of the services.
        let mut tasks: Vec<JoinHandle<()>> = Vec::new();

        // Push a task to run the ready server.
        tasks.push(tokio::spawn(
            self.start_ready_server(health_checks.clone())
                .await
                .context("Failed to create future to start the ready server")?,
        ));

        // Run the node API.
        tasks.push(tokio::spawn(
            self.start_node(test_dir.clone(), node_config)
                .await
                .context("Failed to create future to start the node")?,
        ));

        // If configured, run the faucet.
        if let Some(config) = faucet_config {
            tasks.push(tokio::spawn(
                self.start_faucet(config, node_api_url.clone())
                    .await
                    .context("Failed to create future to start the faucet")?,
            ));
        }

        eprintln!(
            "Readiness endpoint: http://0.0.0.0:{}/\n",
            ready_server_config.ready_server_listen_port
        );

        // Wait for all the services to start up.
        self.wait_for_startup(&health_checks).await?;

        // Wait for all of the futures for the tasks. We should never get past this
        // point unless something goes wrong or the user signals for the process to
        // end.
        let result = futures::future::select_all(tasks).await;

        Err(CliError::UnexpectedError(format!(
            "One of the components stopped unexpectedly: {:?}",
            result
        )))
    }
}
