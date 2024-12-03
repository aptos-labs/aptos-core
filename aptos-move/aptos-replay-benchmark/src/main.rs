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

/// Minimum number of times to execute blocks of transactions and measure the time taken.
const MIN_NUM_REPEATS: usize = 3;

const LONG_ABOUT: &str = "A tool ro replay and benchmark move transactions. \n\
Users can specify ranges of on-chain transactions they want to benchmark, as well as override the \
state (instead of using the on-chain state directly). For example, one can enable new features \
and compare how the execution time or gas usage would change for a sequence of past transactions. \
Note that overriding the state can change execution behaviour. It therefore important to check
the differences between on-chain outputs and outs when the state is overridden. If the differences
are not important (e.g., only gas usage has changed), execution behaviour is still the same. For
example, running the following command:\
\n   aptos-replay-benchmark --begin-version 1944524532 --end-version 1944524714 \\ \
\n                          --rest-endpoint https://mainnet.aptoslabs.com/v1 \\ \
\n                          --concurrency-levels 2 4 \\ \
\n                          --enable-features ENABLE_LOADER_V2 \n\
reruns past transactions from versions 1944524532 to 1944524714 with enabled ENABLE_LOADER_V2 \
feature. It takes two measurements: when Block-STM uses 2 threads, or 4 threads per block.";

#[derive(Parser)]
#[command(about, long_about = Some(LONG_ABOUT))]
pub struct Command {
    #[clap(long, default_value_t = Level::Warn)]
    log_level: Level,

    #[clap(
        long,
        help = "Fullnode's REST API query endpoint, e.g., https://mainnet.aptoslabs.com/v1 for \
                mainnet."
    )]
    rest_endpoint: String,

    #[clap(long, help = "First transaction to include for benchmarking.")]
    begin_version: Version,

    #[clap(long, help = "Last transaction to include for benchmarking.")]
    end_version: Version,

    #[clap(
        long,
        value_delimiter = ' ',
        help = "List of space-separated concurrency levels that define how many threads Block-STM \
                is using to execute a block of transactions. For each level, the time taken to \
                execute blocks of transactions is measured and reported."
    )]
    concurrency_levels: Vec<usize>,

    #[clap(
        long,
        default_value_t = MIN_NUM_REPEATS,
        help = "Number of times to execute blocks of transactions and measure the \
                time taken for each concurrency level."
    )]
    num_repeats: usize,

    #[clap(
        long,
        help = "If true, measure time taken to execute each block, and overall time otherwise."
    )]
    measure_block_time: bool,

    #[clap(
        long,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to enable, in capital letters. For example, \
                GAS_PAYER_ENABLED or EMIT_FEE_STATEMENT. For full list of feature flags, see \
                aptos-core/types/src/on_chain_config/aptos_features.rs."
    )]
    enable_features: Vec<FeatureFlag>,

    #[clap(
        long,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to disable, in capital letters. For \
                example, GAS_PAYER_ENABLED or EMIT_FEE_STATEMENT. For full list of feature flags, \
                see aptos-core/types/src/on_chain_config/aptos_features.rs."
    )]
    disable_features: Vec<FeatureFlag>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::parse();

    let mut logger = Logger::new();
    logger.level(command.log_level);
    logger.init();

    let _mp = MetricsPusher::start(vec![]);

    // Sanity checks for provided commands.
    assert!(
        command.begin_version <= command.end_version,
        "Transaction versions should be a valid closed interval. Instead got begin: {}, end: {}.",
        command.begin_version,
        command.end_version,
    );
    assert!(
        !command.concurrency_levels.is_empty(),
        "At least one concurrency level must be provided with",
    );
    assert!(
        command.num_repeats >= MIN_NUM_REPEATS,
        "Number of repeats must be at least {}",
        MIN_NUM_REPEATS,
    );
    assert!(
        command
            .enable_features
            .iter()
            .all(|f| !command.disable_features.contains(f)),
        "Enable and disable feature flags cannot overlap",
    );

    // TODO:
    //   Right now we fetch transactions from debugger, but ideally we need a way to save them
    //   locally (with corresponding read-sets) so we can use this for CI.
    let debugger = AptosDebugger::rest_client(Client::new(Url::parse(&command.rest_endpoint)?))?;

    // TODO:
    //  Right now, only features can be overridden. In general, this can be allowed for anything:
    //      1. Framework code, e.g., to test performance of new natives or compiler,
    //      2. Gas schedule, to track the costs of charging gas or tracking limits.
    //  We probably should support at least these.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        Command::command().debug_assert();
    }
}
