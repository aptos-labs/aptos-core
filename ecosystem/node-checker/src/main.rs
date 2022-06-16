// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod configuration;
mod evaluator;
mod metric_collector;
mod metric_evaluator;
mod runner;
mod server;
mod system_information_evaluator;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Commands for working with the server.
    Server(server::Server),

    // Commands for working with baseline node configuration.
    Configuration(configuration::Configuration),
}

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct RootArgs {
    #[clap(subcommand)]
    pub command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    let root_args = RootArgs::parse();

    let command = root_args.command;
    let result: Result<()> = match command {
        Command::Server(args) => server::run_cmd(args).await,
        Command::Configuration(args) => configuration::run_cmd(args).await,
    };
    result
}
