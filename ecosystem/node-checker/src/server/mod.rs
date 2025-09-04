// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod api;
mod build;
mod common;
mod generate_openapi;
mod node_information;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};
use generate_openapi::{generate_openapi, GenerateOpenapi};
pub use node_information::NodeInformation;
use run::{run, Run};

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
