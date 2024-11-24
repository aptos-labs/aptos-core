// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::{Level, Logger};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_push_metrics::MetricsPusher;
use aptos_replay_benchmark::{
    generator::BenchmarkGenerator, overrides::OverrideConfig, runner::BenchmarkRunner,
};
use aptos_rest_client::Client;
use aptos_types::{on_chain_config::FeatureFlag, transaction::Version};
use clap::Parser;
use url::Url;

#[derive(Parser)]
pub struct Command {
    #[clap(long, help = "Logging level, defaults to ERROR")]
    log_level: Option<Level>,

    #[clap(long, help = "Fullnode's REST API query endpoint")]
    rest_endpoint: String,

    #[clap(long, help = "First transaction to include for benchmarking")]
    begin_version: Version,

    #[clap(long, help = "Last transaction to include for benchmarking")]
    end_version: Version,

    #[clap(
        long,
        num_args=1..,
        value_delimiter = ' ',
        help = "Different concurrency levels to benchmark",
    )]
    concurrency_levels: Vec<usize>,

    #[clap(
        long,
        help = "Number of times to repeat an experiment for each concurrency level"
    )]
    num_repeats: Option<usize>,

    #[clap(
        long,
        help = "If true, measure time taken to execute each block, and overall time otherwise"
    )]
    measure_block_time: bool,

    #[clap(
        long,
        num_args=1..,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to enable",
    )]
    enable_features: Vec<FeatureFlag>,

    #[clap(
        long,
        num_args=1..,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to disable",
    )]
    disable_features: Vec<FeatureFlag>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::parse();

    Logger::new()
        .level(command.log_level.unwrap_or(Level::Error))
        .init();
    let _mp = MetricsPusher::start(vec![]);

    let debugger = AptosDebugger::rest_client(Client::new(Url::parse(&command.rest_endpoint)?))?;
    let override_config = OverrideConfig::new(command.enable_features, command.disable_features);

    let blocks = BenchmarkGenerator::new(
        debugger,
        command.begin_version,
        command.end_version,
        override_config,
    )
    .generate_blocks()
    .await?;

    for block in &blocks {
        block.print_diffs();
    }

    BenchmarkRunner::new(
        command.concurrency_levels,
        command.num_repeats,
        command.measure_block_time,
    )
    .measure_execution_time(&blocks);

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Command::command().debug_assert();
}
