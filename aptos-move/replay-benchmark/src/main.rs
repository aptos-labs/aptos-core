// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_replay_benchmark::commands::{BenchmarkCommand, DownloadCommand, InitializeCommand};
use clap::Parser;

#[derive(Parser)]
#[command(about)]
pub enum Command {
    Download(DownloadCommand),
    Initialize(InitializeCommand),
    Benchmark(BenchmarkCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::parse();
    match command {
        Command::Download(command) => command.download_and_save_transactions().await,
        Command::Initialize(command) => command.initialize_inputs_for_workloads().await,
        Command::Benchmark(command) => command.benchmark().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        Command::command().debug_assert();
    }
}
