// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod run;
mod server_config;
mod validate_config;

use anyhow::Result;
use clap::Subcommand;
use run::Run;
use validate_config::ValidateConfig;

#[derive(Clone, Debug, Subcommand)]
pub enum Server {
    /// Run the server.
    Run(Run),

    /// Confirm a server config is valid.
    ValidateConfig(ValidateConfig),
}

impl Server {
    pub async fn run_command(&self) -> Result<()> {
        match self {
            Server::Run(args) => args.run().await,
            Server::ValidateConfig(args) => args.validate_config().await,
        }
    }
}
