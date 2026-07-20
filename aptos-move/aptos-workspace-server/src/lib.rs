// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This library runs and manages a set of services that makes up a local Aptos network.
//! - node
//!     - node API
//! - faucet
//!
//! The services are bound to unique OS-assigned ports to allow for multiple local networks
//! to operate simultaneously, enabling testing and development in isolated environments.
//!
//! ## Key Features:
//! - Shared Futures
//!     - The code makes extensive use of shared futures across multiple services,
//!       ensuring orderly startup while maximizing parallel execution.
//! - Graceful Shutdown
//!     - When a `Ctrl-C` signal is received or if any of the services fail to start
//!       or exit unexpectedly, the system attempts to gracefully shut down all services.

mod common;
mod services;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use common::make_shared;
use futures::TryFutureExt;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
enum ControlCommand {
    Stop,
}

/// Starts an async task that reads and processes commands from stdin.
///
/// Currently there is only one command:
/// - stop: shuts down the services gracefully.
async fn start_command_processor(shutdown: CancellationToken) {
    let reader = BufReader::new(tokio::io::stdin());
    let mut lines = reader.lines();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(input)) => {
                        let res = ControlCommand::try_parse_from(format!("dummy {}", input).split_whitespace());
                        // Note: clap expects a program name so we add a dummy one.
                        match res {
                            Ok(cmd) => match cmd {
                                ControlCommand::Stop => {
                                    no_panic_println!("\nStop command received. Shutting down services. This may take a while.\n");
                                    shutdown.cancel();
                                    break;
                                }
                            }
                            Err(_) => {
                                no_panic_eprintln!("Invalid command: \"{}\"", input);
                            }
                        }
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(err) => {
                        no_panic_eprintln!("Error reading from stdin: {}", err);
                        break;
                    }
                }
            }
            _ = shutdown.cancelled() => {
                break;
            }
        }
    }
}

async fn run_all_services(timeout: u64) -> Result<()> {
    let test_dir = tempfile::tempdir()?;
    let test_dir = test_dir.path();
    no_panic_println!("Created test directory: {}", test_dir.display());

    // Phase 0: Start the background services

    // Register the signal handler for ctrl-c.
    let shutdown = CancellationToken::new();
    {
        // TODO: Find a way to register the signal handler in a blocking manner without
        //       waiting for it to trigger.
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            tokio::select! {
                res = tokio::signal::ctrl_c() => {
                    res.unwrap();
                    no_panic_println!("\nCtrl-C received. Shutting down services. This may take a while.\n");
                }
                _ = tokio::time::sleep(Duration::from_secs(timeout)) => {
                    no_panic_println!("\nTimeout reached. Shutting down services. This may take a while.\n");
                }
            }

            shutdown.cancel();
        });
    }

    // Start the command processor.
    {
        let shutdown = shutdown.clone();
        tokio::spawn(start_command_processor(shutdown));
    }

    // Phase 1: Start all services.
    // Node
    let (fut_node_api, fut_node_finish) = services::node::start_node(test_dir)?;

    let fut_node_api = make_shared(fut_node_api);

    // Faucet
    let (fut_faucet, fut_faucet_finish) =
        services::faucet::start_faucet(test_dir.to_owned(), fut_node_api.clone());

    // Phase 2: Wait for all services to be up.
    let all_services_up =
        async move { tokio::try_join!(fut_node_api.map_err(|err| anyhow!(err)), fut_faucet,) };
    tokio::select! {
        _ = shutdown.cancelled() => {
            return Ok(())
        }
        res = all_services_up => {
            match res.context("one or more services failed to start") {
                Ok(_) => no_panic_println!("ALL SERVICES UP"),
                Err(err) => {
                    no_panic_eprintln!("\nOne or more services failed to start\n");
                    return Err(err)
                }
            }
        }
    }

    // Phase 3: Wait for services to stop, which should only happen in case of an error, or
    //          the shutdown signal to be received.
    tokio::select! {
        _ = shutdown.cancelled() => (),
        res = fut_node_finish => {
            no_panic_eprintln!("Node exited unexpectedly");
            if let Err(err) = res {
                no_panic_eprintln!("Error: {}", err);
            }
        }
        res = fut_faucet_finish => {
            no_panic_eprintln!("Faucet exited unexpectedly");
            if let Err(err) = res {
                no_panic_eprintln!("Error: {}", err);
            }
        }
    }

    no_panic_println!("Finished running all services");

    Ok(())
}

#[derive(Parser)]
pub enum WorkspaceCommand {
    Run {
        #[arg(long, default_value_t = 1800)]
        timeout: u64,
    },
}

impl WorkspaceCommand {
    pub async fn run(self) -> Result<()> {
        match self {
            WorkspaceCommand::Run { timeout } => run_all_services(timeout).await,
        }
    }
}
