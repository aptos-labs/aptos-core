// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
