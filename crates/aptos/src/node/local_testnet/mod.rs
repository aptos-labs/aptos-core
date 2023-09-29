// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod faucet;
mod health_checker;
mod indexer_api;
mod logging;
mod node;
mod postgres;
mod processors;
mod ready_server;
mod traits;
mod utils;

use self::{
    faucet::FaucetArgs,
    health_checker::HealthChecker,
    indexer_api::IndexerApiArgs,
    logging::ThreadNameMakeWriter,
    node::NodeArgs,
    postgres::PostgresArgs,
    processors::ProcessorArgs,
    ready_server::ReadyServerArgs,
    traits::{PostHealthyStep, ServiceManager},
};
use crate::{
    common::{
        types::{CliCommand, CliError, CliTypedResult, ConfigSearchMode, PromptOptions},
        utils::prompt_yes_with_override,
    },
    config::GlobalConfig,
    node::local_testnet::{
        faucet::FaucetManager, indexer_api::IndexerApiManager, node::NodeManager,
        processors::ProcessorManager, ready_server::ReadyServerManager, traits::ShutdownStep,
    },
};
use anyhow::Context;
use aptos_indexer_grpc_server_framework::setup_logging;
use async_trait::async_trait;
use clap::Parser;
use std::{
    collections::HashSet,
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::task::JoinHandle;
use tracing::{info, warn};
use tracing_subscriber::fmt::MakeWriter;

const TESTNET_FOLDER: &str = "testnet";

/// Run a local testnet
///
/// This local testnet will run it's own genesis and run as a single node network
/// locally. A faucet and grpc transaction stream will run alongside the node unless
/// you specify otherwise with --no-faucet and --no-txn-stream respectively.
#[derive(Parser)]
pub struct RunLocalTestnet {
    /// The directory to save all files for the node
    ///
    /// Defaults to .aptos/testnet
    #[clap(long, value_parser)]
    test_dir: Option<PathBuf>,

    /// Clean the state and start with a new chain at genesis
    ///
    /// This will wipe the aptosdb in `--test-dir` to remove any incompatible changes, and start
    /// the chain fresh. Note, that you will need to publish the module again and distribute funds
    /// from the faucet accordingly.
    #[clap(long)]
    force_restart: bool,

    #[clap(flatten)]
    node_args: NodeArgs,

    #[clap(flatten)]
    faucet_args: FaucetArgs,

    #[clap(flatten)]
    postgres_args: PostgresArgs,

    #[clap(flatten)]
    processor_args: ProcessorArgs,

    #[clap(flatten)]
    indexer_api_args: IndexerApiArgs,

    #[clap(flatten)]
    ready_server_args: ReadyServerArgs,

    #[clap(flatten)]
    prompt_options: PromptOptions,
}

impl RunLocalTestnet {
    /// Wait for many services to start up. This prints a message like "X is starting,
    /// please wait..." for each service and then "X is ready. Endpoint: <url>"
    /// when it's ready.
    async fn wait_for_startup<'a>(
        &self,
        health_checkers: &HashSet<HealthChecker>,
        test_dir: &Path,
    ) -> CliTypedResult<()> {
        let mut futures: Vec<Pin<Box<dyn futures::Future<Output = anyhow::Result<()>> + Send>>> =
            Vec::new();

        for health_checker in health_checkers {
            let silent = match health_checker {
                HealthChecker::NodeApi(_) => false,
                // We don't want to print anything for the processors, it'd be too spammy.
                HealthChecker::Http(_, name) => name.contains("processor"),
                HealthChecker::DataServiceGrpc(_) => false,
                HealthChecker::Postgres(_) => false,
            };
            if !silent {
                eprintln!("{} is starting, please wait...", health_checker);
            }
            let fut = async move {
                health_checker.wait(None).await?;
                if !silent {
                    eprintln!(
                        "{} is ready. Endpoint: {}",
                        health_checker,
                        health_checker.address_str()
                    );
                }
                Ok(())
            };
            futures.push(Box::pin(fut));
        }

        eprintln!();

        // We use join_all because we expect all of these to return.
        for f in futures::future::join_all(futures).await {
            f.map_err(|err| {
                CliError::UnexpectedError(format!(
                    "One of the services failed to start up: {:?}. \
                    Please check the logs at {} for more information.",
                    err,
                    test_dir.display(),
                ))
            })?;
        }

        Ok(())
    }
}

#[async_trait]
impl CliCommand<()> for RunLocalTestnet {
    fn command_name(&self) -> &'static str {
        "RunLocalTestnet"
    }

    fn jsonify_error_output(&self) -> bool {
        false
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
        if self.force_restart && test_dir.exists() {
            prompt_yes_with_override(
                "Are you sure you want to delete the existing local testnet data?",
                self.prompt_options,
            )?;
            remove_dir_all(test_dir.as_path()).map_err(|err| {
                CliError::IO(format!("Failed to delete {}", test_dir.display()), err)
            })?;
            info!("Deleted test directory at: {:?}", test_dir);
        }

        if !test_dir.exists() {
            info!("Test directory does not exist, creating it: {:?}", test_dir);
            create_dir_all(test_dir.as_path()).map_err(|err| {
                CliError::IO(format!("Failed to create {}", test_dir.display()), err)
            })?;
            info!("Created test directory: {:?}", test_dir);
        }

        // Set up logging for anything that uses tracing. These logs will go to
        // different directories based on the name of the runtime.
        let td = test_dir.clone();
        let make_writer =
            move || ThreadNameMakeWriter::new(td.clone()).make_writer() as Box<dyn std::io::Write>;
        setup_logging(Some(Box::new(make_writer)));

        let mut managers: Vec<Box<dyn ServiceManager>> = Vec::new();

        // Build the node manager. We do this unconditionally.
        let node_manager = NodeManager::new(&self, test_dir.clone())
            .context("Failed to build node service manager")?;
        let node_health_checkers = node_manager.get_healthchecks();

        // If configured to do so, build the faucet manager.
        if !self.faucet_args.no_faucet {
            let faucet_manager = FaucetManager::new(
                &self,
                node_health_checkers.clone(),
                test_dir.clone(),
                node_manager.get_node_api_url(),
            )
            .context("Failed to build faucet service manager")?;
            managers.push(Box::new(faucet_manager));
        }

        if self.indexer_api_args.with_indexer_api {
            let postgres_manager = postgres::PostgresManager::new(&self, test_dir.clone())
                .context("Failed to build postgres service manager")?;
            let postgres_health_checkers = postgres_manager.get_healthchecks();
            managers.push(Box::new(postgres_manager));

            let processor_preqrequisite_healthcheckers =
                [node_health_checkers, postgres_health_checkers]
                    .into_iter()
                    .flatten()
                    .collect();
            let processor_managers = ProcessorManager::many_new(
                &self,
                processor_preqrequisite_healthcheckers,
                node_manager.get_data_service_url(),
                self.postgres_args.get_connection_string(None),
            )
            .context("Failed to build processor service managers")?;

            // We have already ensured that at least one processor is used when
            // building the processor managers with `many_new`.
            let processor_health_checkers = processor_managers[0].get_healthchecks();

            let mut processor_managers = processor_managers
                .into_iter()
                .map(|m| Box::new(m) as Box<dyn ServiceManager>)
                .collect();
            managers.append(&mut processor_managers);

            let indexer_api_manager = IndexerApiManager::new(
                &self,
                processor_health_checkers,
                test_dir.clone(),
                self.postgres_args.get_connection_string(None),
            )
            .context("Failed to build indexer API service manager")?;
            managers.push(Box::new(indexer_api_manager));
        }

        // We put the node manager into managers at the end just so we have access to
        // it before this so we can call things like `node_manager.get_node_api_url()`.
        managers.push(Box::new(node_manager));

        // Get the healthcheckers from all the managers. We'll pass to this
        // `wait_for_startup`.
        let health_checkers: HashSet<HealthChecker> =
            managers.iter().flat_map(|m| m.get_healthchecks()).collect();

        // The final manager we add is the ready server. This must happen last since
        // it use the health checkers from all the other services.
        managers.push(Box::new(ReadyServerManager::new(
            &self,
            health_checkers.clone(),
        )?));

        // Collect steps to run on shutdown. We run these in reverse.
        let shutdown_steps: Vec<Box<dyn ShutdownStep>> = managers
            .iter()
            .flat_map(|m| m.get_shutdown_steps())
            .rev()
            .collect();

        // Run any pre-run steps.
        for manager in &managers {
            manager.pre_run().await.with_context(|| {
                format!("Failed to apply pre run steps for {}", manager.get_name())
            })?;
        }

        eprintln!(
            "Readiness endpoint: http://0.0.0.0:{}/\n",
            self.ready_server_args.ready_server_listen_port,
        );

        // Collect post healthy steps to run after the services start.
        let post_healthy_steps: Vec<Box<dyn PostHealthyStep>> = managers
            .iter()
            .flat_map(|m| m.get_post_healthy_steps())
            .collect();

        let mut tasks: Vec<JoinHandle<()>> = Vec::new();

        // Start each of the services.
        for manager in managers.into_iter() {
            tasks.push(manager.run());
        }

        // Wait for all the services to start up.
        self.wait_for_startup(&health_checkers, &test_dir).await?;

        eprintln!("\nApplying post startup steps...");

        // Run any post healthy steps.
        for post_healthy_step in post_healthy_steps {
            post_healthy_step
                .run()
                .await
                .context("Failed to run post startup step")?;
        }

        eprintln!("\nSetup is complete, you can now use the local testnet!");

        // Create a task that listens for ctrl-c. We want to intercept it so we can run
        // the shutdown steps before properly exiting. This is of course best effort,
        // see `ShutdownStep` for more info. In particular, to speak to how "best effort"
        // this really is, to make sure ctrl-c happens more or less instantly, we only
        // register this handler after all the services have started.
        tasks.push(tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register ctrl-c hook");
        }));

        // Wait for all of the tasks. We should never get past this point unless
        // something goes goes wrong or the user signals for the process to end.
        let num_tasks = tasks.len();
        let (_, finished_future_index, _) = futures::future::select_all(tasks).await;

        // Because we added the ctrl-c task last, we can figure out if that was the one
        // that ended based on `finished_future_index`. We modify our messaging and the
        // return value based on this.
        let was_ctrl_c = finished_future_index == num_tasks - 1;
        if was_ctrl_c {
            eprintln!("\nReceived ctrl-c, running shutdown steps...");
        } else {
            eprintln!("\nOne of the futures exited unexpectedly, running shutdown steps...");
        }

        // At this point replace the ctrl-c handler so the user can kill the CLI
        // instantly if they send the signal twice.
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register ctrl-c hook");
            warn!("Received ctrl-c twice and exited immediately");
            eprintln!();
            std::process::exit(1);
        });

        // Run shutdown steps, if any.
        for shutdown_step in shutdown_steps {
            shutdown_step
                .run()
                .await
                .context("Failed to run shutdown step")?;
        }

        eprintln!("Done, goodbye!");

        match was_ctrl_c {
            true => Ok(()),
            false => Err(CliError::UnexpectedError(
                "One of the services stopped unexpectedly".to_string(),
            )),
        }
    }
}
