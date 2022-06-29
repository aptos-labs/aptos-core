// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod api;
mod common;
mod configurations_manager;
mod generate_openapi;
mod node_information;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};
use generate_openapi::{generate_openapi, GenerateOpenapi};
use run::{run, Run};

pub use node_information::NodeInformation;

#[derive(Clone, Debug, Parser)]
pub struct Server {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// todo
    Run(Run),

    /// todo
    GenerateOpenapi(GenerateOpenapi),
}

pub async fn run_cmd(args: Server) -> Result<()> {
    let result: Result<()> = match args.cmd {
        Command::Run(args) => run(args).await,
        Command::GenerateOpenapi(args) => generate_openapi(args).await,
    };
    result
}
