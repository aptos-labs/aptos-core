// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_node_checker_lib::{configuration, server};
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

    aptos_logger::Logger::builder()
        .level(aptos_logger::Level::Info)
        .build();

    let command = root_args.command;
    let result: Result<()> = match command {
        Command::Server(args) => server::run_cmd(args).await,
        Command::Configuration(args) => configuration::run_cmd(args).await,
    };
    result
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    RootArgs::command().debug_assert()
}
