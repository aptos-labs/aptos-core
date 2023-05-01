// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_api_gateway::server::Server;
use aptos_logger::info;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    server: Server,
}

impl Args {
    pub async fn run_command(&self) -> Result<()> {
        self.server.run_command().await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let root_args = Args::parse();

    aptos_logger::Logger::builder()
        .level(aptos_logger::Level::Info)
        .build();

    info!("Running with root args: {:#?}", root_args);

    root_args.run_command().await
}
