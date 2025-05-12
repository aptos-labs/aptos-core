// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod logging;
mod postgres;
mod ready_server;
mod utils;

// This is to allow external crates to use the localnode.
pub mod docker;
pub mod faucet;
pub mod health_checker;
pub mod indexer_api;
pub mod node;
pub mod processors;
pub mod traits;

use self::{
    faucet::FaucetArgs,
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
use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::setup_logging;
use async_trait::async_trait;
use clap::Parser;
pub use health_checker::HealthChecker;
use std::{
    collections::HashSet,
    fs::{create_dir_all, remove_dir_all},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::task::JoinSet;
use tracing::{info, warn};
use tracing_subscriber::fmt::MakeWriter;

const TESTNET_FOLDER: &str = "testnet";

/// Run a localnet
///
/// This localnet will run it's own genesis and run as a single node network
/// locally. A faucet and grpc transaction stream will run alongside the node unless
/// you specify otherwise with --no-faucet and --no-txn-stream respectively.
#[derive(Parser)]
pub struct RunLocalnet {
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

    /// By default all services running on the host system will be bound to 127.0.0.1,
    /// unless you're running the CLI inside a container, in which case it will run
    /// them on 0.0.0.0. You can use this flag to override this behavior in both cases.
    #[clap(long)]
    bind_to: Option<Ipv4Addr>,

    /// By default, tracing output goes to files. With this set, it goes to stdout.
    #[clap(long, hide = true)]
    log_to_stdout: bool,
}

impl RunLocalnet {
    /// Wait for many services to start up. This prints a message like "X is starting,
    /// please wait..." for each service and then "X is ready. Endpoint: <url>"
    /// when it's ready.
    async fn wait_for_startup(
        &self,
        health_checkers: &HashSet<HealthChecker>,
        test_dir: &Path,
    ) -> CliTypedResult<()> {
        let mut futures: Vec<Pin<Box<dyn futures::Future<Output = anyhow::Result<()>> + Send>>> =
            Vec::new();

        for health_checker in health_checkers {
            let silent = match health_checker {
                HealthChecker::NodeApi(_) => false,
                HealthChecker::Http(_, _) => false,
                HealthChecker::DataServiceGrpc(_) => false,
                HealthChecker::Postgres(_) => false,
                // We don't want to print anything for the processors, it'd be too spammy.
                HealthChecker::Processor(_, _) => true,
                // We don't want to actually wait on this health checker here because
                // it will never return true since we apply the metadata in a post
                // healthy step (which comes after we call this function). So we move
                // on. This is a bit of a leaky abstraction that we can solve with more
                // lifecycle hooks down the line.
                HealthChecker::IndexerApiMetadata(_) => continue,
            };
            if !silent {
                println!("{} is starting, please wait...", health_checker);
            } else {
                info!("[silent] {} is starting, please wait...", health_checker);
            }
            let fut = async move {
                health_checker.wait(None).await?;
                if !silent {
                    println!(
                        "{} is ready. Endpoint: {}",
                        health_checker,
                        health_checker.address_str()
                    );
                } else {
                    info!(
                        "[silent] {} is ready. Endpoint: {}",
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
impl CliCommand<()> for RunLocalnet {
    fn command_name(&self) -> &'static str {
        "RunLocalnet"
    }

    fn jsonify_error_output(&self) -> bool {
        false
    }

    async fn execute(mut self) -> CliTypedResult<()> {
        if self.log_to_stdout {
            setup_logging(None);
        }

        // Based on the input and global config, get the test directory.
        let test_dir = get_derived_test_dir(&self.test_dir)?;

        // If asked, remove the current test directory and start with a new node.
        if self.force_restart && test_dir.exists() {
            prompt_yes_with_override(
                "Are you sure you want to delete the existing localnet data?",
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

        // We set up directory based logging after we have created test_dir.
        if !self.log_to_stdout {
            // Set up logging for anything that uses tracing. These logs will go to
            // different directories based on the name of the runtime.
            let td = test_dir.clone();
            let make_writer = move || {
                ThreadNameMakeWriter::new(td.clone()).make_writer() as Box<dyn std::io::Write>
            };
            setup_logging(Some(Box::new(make_writer)));
        }

        // If the CLI is running inside a container, bind services not running inside a
        // container to 0.0.0.0 (so they can be accessed from outside the container).
        // Otherwise bind them to 127.0.0.1. This is necessary because Windows
        // complains about services binding to 0.0.0.0 sometimes.
        let running_inside_container = Path::new(".dockerenv").exists();
        let bind_to = match self.bind_to {
            Some(bind_to) => bind_to,
            None => {
                if running_inside_container {
                    Ipv4Addr::new(0, 0, 0, 0)
                } else {
                    Ipv4Addr::new(127, 0, 0, 1)
                }
            },
        };
        info!("Binding host services to {}", bind_to);

        let mut managers: Vec<Box<dyn ServiceManager>> = Vec::new();

        // Build the node manager. We do this unconditionally.
        let node_manager = NodeManager::new(&self, bind_to, test_dir.clone())
            .context("Failed to build node service manager")?;
        let node_health_checkers = node_manager.get_health_checkers();

        // If configured to do so, build the faucet manager.
        if !self.faucet_args.no_faucet {
            let faucet_manager = FaucetManager::new(
                &self,
                node_health_checkers.clone(),
                bind_to,
                test_dir.clone(),
                node_manager.get_node_api_url(),
            )
            .context("Failed to build faucet service manager")?;
            managers.push(Box::new(faucet_manager));
        }

        if self.indexer_api_args.with_indexer_api {
            let postgres_manager = postgres::PostgresManager::new(&self, test_dir.clone())
                .context("Failed to build postgres service manager")?;
            let postgres_health_checkers = postgres_manager.get_health_checkers();
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
                self.postgres_args.get_connection_string(None, true),
            )
            .context("Failed to build processor service managers")?;

            let processor_health_checkers = processor_managers
                .iter()
                .flat_map(|m| m.get_health_checkers())
                .collect();

            let mut processor_managers = processor_managers
                .into_iter()
                .map(|m| Box::new(m) as Box<dyn ServiceManager>)
                .collect();
            managers.append(&mut processor_managers);

            let indexer_api_manager = IndexerApiManager::new(
                &self,
                processor_health_checkers,
                test_dir.clone(),
                self.postgres_args.get_connection_string(None, false),
            )
            .context("Failed to build indexer API service manager")?;
            managers.push(Box::new(indexer_api_manager));
        }

        // We put the node manager into managers at the end just so we have access to
        // it before this so we can call things like `node_manager.get_node_api_url()`.
        managers.push(Box::new(node_manager));

        // Get the healthcheckers from all the managers. We'll pass to this
        // `wait_for_startup`.
        let health_checkers: HashSet<HealthChecker> = managers
            .iter()
            .flat_map(|m| m.get_health_checkers())
            .collect();

        // The final manager we add is the ready server. This must happen last since
        // it use the health checkers from all the other services.
        managers.push(Box::new(ReadyServerManager::new(
            &self,
            bind_to,
            health_checkers.clone(),
        )?));

        // Collect steps to run on shutdown. We run these in reverse. This is somewhat
        // arbitrary, each shutdown step should work no matter the order it is run in.
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

        println!(
            "\nReadiness endpoint: http://{}:{}/\n",
            bind_to, self.ready_server_args.ready_server_listen_port,
        );

        // Collect post healthy steps to run after the services start.
        let post_healthy_steps: Vec<Box<dyn PostHealthyStep>> = managers
            .iter()
            .flat_map(|m| m.get_post_healthy_steps())
            .collect();

        let mut join_set = JoinSet::new();

        // Start each of the services.
        for manager in managers.into_iter() {
            join_set.spawn(manager.run());
        }

        // Wait for all the services to start up. While doing so we also wait for any
        // of the services to end. This is not meant to ever happen (except for ctrl-c,
        // which we don't catch yet, so the process will just abort). So if it does
        // happen, it means one of the services failed to start up, in which case we
        // stop waiting for the rest of the services and error out.
        tokio::select! {
            res = self.wait_for_startup(&health_checkers, &test_dir) => {
                res?
            },
            res = join_set.join_next() => {
                eprintln!("\nOne of the services failed to start up, running shutdown steps...");
                run_shutdown_steps(shutdown_steps).await?;
                eprintln!("Ran shutdown steps");
                return Err(CliError::UnexpectedError(format!(
                    "\nOne of the services crashed on startup:\n{:#?}\nPlease check the logs in {}",
                    // We can unwrap because we know for certain that the JoinSet is
                    // not empty.
                    res.unwrap(),
                    test_dir.display(),
                )));
            }
        }

        eprintln!("\nApplying post startup steps...");

        // Run any post healthy steps.
        for post_healthy_step in post_healthy_steps {
            post_healthy_step
                .run()
                .await
                .context("Failed to run post startup step")?;
        }

        eprintln!("\nSetup is complete, you can now use the localnet!");

        // Create a task that listens for ctrl-c. We want to intercept it so we can run
        // the shutdown steps before properly exiting. This is of course best effort,
        // see `ShutdownStep` for more info. In particular, to speak to how "best effort"
        // this really is, to make sure ctrl-c happens more or less instantly, we only
        // register this handler after all the services have started.
        let abort_handle = join_set.spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register ctrl-c hook");
            Ok(())
        });
        let ctrl_c_task_id = abort_handle.id();

        // Wait for one of the tasks to end. We should never get past this point unless
        // something goes goes wrong or the user signals for the process to end. We
        // unwrap once because we know for certain the set is not empty and that's the
        // only condition in which this can return `None`.
        let result = join_set.join_next_with_id().await.unwrap();

        // We want to print a different message depending on which task ended. We can
        // determine if the task that ended was the ctrl-c task based on the ID of the
        // task.
        let finished_task_id = match &result {
            Ok((id, _)) => *id,
            Err(err) => err.id(),
        };

        let was_ctrl_c = finished_task_id == ctrl_c_task_id;
        if was_ctrl_c {
            eprintln!("\nReceived ctrl-c, running shutdown steps...");
        } else {
            eprintln!("\nOne of the services exited unexpectedly, running shutdown steps...");
        }

        // At this point register another ctrl-c handler so the user can kill the CLI
        // instantly if they send the signal twice.
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register ctrl-c hook");
            warn!("Received ctrl-c twice and exited immediately");
            eprintln!();
            std::process::exit(1);
        });

        // Run post shutdown steps, if any.
        run_shutdown_steps(shutdown_steps).await?;

        eprintln!("Done, goodbye!");

        match was_ctrl_c {
            true => Ok(()),
            false => Err(CliError::UnexpectedError(format!(
                "One of the services stopped unexpectedly.\nPlease check the logs in {}",
                test_dir.display()
            ))),
        }
    }
}

async fn run_shutdown_steps(shutdown_steps: Vec<Box<dyn ShutdownStep>>) -> Result<()> {
    for shutdown_step in shutdown_steps {
        shutdown_step
            .run()
            .await
            .context("Failed to run shutdown step")?;
    }
    Ok(())
}

pub fn get_derived_test_dir(input_test_dir: &Option<PathBuf>) -> Result<PathBuf> {
    let global_config = GlobalConfig::load().context("Failed to load global config")?;
    match input_test_dir {
        Some(test_dir) => Ok(test_dir.clone()),
        None => Ok(global_config
            .get_config_location(ConfigSearchMode::CurrentDirAndParents)?
            .join(TESTNET_FOLDER)),
    }
}
