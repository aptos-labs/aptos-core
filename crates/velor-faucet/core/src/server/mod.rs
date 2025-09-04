// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod generate_openapi;
mod run;
mod server_args;
mod validate_config;

use anyhow::Result;
use clap::Subcommand;
use generate_openapi::GenerateOpenapi;
pub use run::{FunderKeyEnum, RunConfig};
use run::{Run, RunSimple};
use validate_config::ValidateConfig;

#[derive(Clone, Debug, Subcommand)]
pub enum Server {
    /// Run the server.
    Run(Run),

    /// Run the server but instead of taking in a config, take in arguments on the CLI.
    /// This is less expressive than Run and is only intended for use alongside local
    /// testnets.
    RunSimple(RunSimple),

    /// Confirm a server config is valid.
    ValidateConfig(ValidateConfig),

    /// Generate the OpenAPI spec.
    GenerateOpenapi(GenerateOpenapi),
}

impl Server {
    pub async fn run_command(&self) -> Result<()> {
        let result: Result<()> = match self {
            Server::Run(args) => args.run().await,
            Server::RunSimple(args) => args.run_simple().await,
            Server::GenerateOpenapi(args) => args.generate_openapi().await,
            Server::ValidateConfig(args) => args.validate_config().await,
        };
        result
    }
}
