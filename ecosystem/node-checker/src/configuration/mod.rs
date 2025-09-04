// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod common;
mod node_address;
mod types;
mod validate;

use anyhow::Result;
use clap::{Parser, Subcommand};
pub use common::read_configuration_from_file;
pub use node_address::NodeAddress;
pub use types::BaselineConfiguration;
use validate::{validate, Validate};

#[derive(Clone, Debug, Parser)]
pub struct Configuration {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Validate a baseline configuration.
    Validate(Validate),
}

pub async fn run_cmd(args: Configuration) -> Result<()> {
    let result: Result<()> = match args.cmd {
        Command::Validate(args) => validate(args).await,
    };
    result
}
