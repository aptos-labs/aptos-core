// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod common;
mod create;
mod types;
mod validate;

use anyhow::Result;
use clap::{Parser, Subcommand};

use create::{create, Create};
use validate::{validate, Validate};

pub use common::read_configuration_from_file;
pub use types::{
    EvaluatorArgs, NodeAddress, NodeConfiguration, DEFAULT_API_PORT, DEFAULT_API_PORT_STR,
    DEFAULT_METRICS_PORT, DEFAULT_METRICS_PORT_STR, DEFAULT_NOISE_PORT, DEFAULT_NOISE_PORT_STR,
};

#[derive(Clone, Debug, Parser)]
pub struct Configuration {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Create a new baseline configuration.
    Create(Create),

    /// Validate an existing baseline configuration.
    Validate(Validate),
}

pub async fn run_cmd(args: Configuration) -> Result<()> {
    let result: Result<()> = match args.cmd {
        Command::Create(args) => create(args).await,
        Command::Validate(args) => validate(args).await,
    };
    result
}
