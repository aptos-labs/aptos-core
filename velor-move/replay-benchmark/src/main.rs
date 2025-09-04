// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_replay_benchmark::commands::{
    BenchmarkCommand, DiffCommand, DownloadCommand, InitializeCommand,
};
use clap::Parser;

#[derive(Parser)]
#[command(about)]
pub enum Command {
    Download(DownloadCommand),
    Initialize(InitializeCommand),
    Diff(DiffCommand),
    Benchmark(BenchmarkCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::parse();
    match command {
        Command::Download(command) => command.download_transactions().await,
        Command::Initialize(command) => command.initialize_inputs().await,
        Command::Diff(command) => command.diff_outputs().await,
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
