// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::{Level, Logger};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_push_metrics::MetricsPusher;
use aptos_replay_benchmark::{AptosBenchmarkRunner, ClosedInterval, EnvironmentOverride};
use aptos_rest_client::Client;
use aptos_types::on_chain_config::FeatureFlag;
use clap::Parser;
use url::Url;

#[derive(Parser)]
pub struct Command {
    #[clap(long, help = "Logging level, defaults to ERROR")]
    log_level: Option<Level>,

    #[clap(long, help = "Fullnode's REST API query endpoint")]
    rest_endpoint: String,

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

    #[clap(long, help = "First transaction to include for benchmarking")]
    begin_version: u64,

    #[clap(long, help = "Last transaction to include for benchmarking")]
    end_version: u64,

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

    #[clap(long, help = "If specified, used as the gas feature version")]
    gas_feature_version: Option<u64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::parse();

    let level = command.log_level.unwrap_or(Level::Error);
    Logger::new().level(level).init();
    let _mp = MetricsPusher::start(vec![]);

    let debugger = AptosDebugger::rest_client(Client::new(Url::parse(&command.rest_endpoint)?))?;
    let versions = ClosedInterval::new(command.begin_version, command.end_version);
    let environment_override = EnvironmentOverride::new(
        command.enable_features,
        command.disable_features,
        command.gas_feature_version,
    );

    let runner = AptosBenchmarkRunner::new(
        debugger,
        versions,
        command.concurrency_levels,
        command.num_repeats,
        environment_override,
    );
    runner.benchmark_past_transactions().await?;

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Command::command().debug_assert();
}
