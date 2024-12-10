// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::{Level, Logger};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_push_metrics::MetricsPusher;
use aptos_replay_benchmark::{
    generator::BenchmarkGenerator, overrides::OverrideConfig, runner::BenchmarkRunner,
};
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_types::{on_chain_config::FeatureFlag, transaction::Version};
use clap::Parser;
use url::Url;

/// Minimum number of times to execute blocks of transactions and measure the time taken.
const MIN_NUM_REPEATS: usize = 3;

#[derive(Parser)]
#[command(about)]
pub struct Command {
    #[clap(long, default_value_t = Level::Error)]
    log_level: Level,

    #[clap(
        long,
        help = "Fullnode's REST API query endpoint, e.g., https://mainnet.aptoslabs.com/v1 for \
                mainnet."
    )]
    rest_endpoint: String,

    #[clap(
        long,
        help = "Optional API key to increase HTTP request rate limit quota."
    )]
    api_key: Option<String>,

    #[clap(long, help = "First transaction to include for benchmarking.")]
    begin_version: Version,

    #[clap(long, help = "Last transaction to include for benchmarking.")]
    end_version: Version,

    #[clap(
        long,
        default_value_t = 0,
        help = "Number of blocks to skip for time measurement. Allows to warm-up caches."
    )]
    num_blocks_to_skip: usize,

    #[clap(
        long,
        num_args = 1..,
        help = "List of space-separated concurrency levels that define how many threads Block-STM \
                is using to execute a block of transactions. For each level, the time taken to \
                execute blocks of transactions is measured and reported."
    )]
    concurrency_levels: Vec<usize>,

    #[clap(
        long,
        default_value_t = MIN_NUM_REPEATS,
        help = "Number of times to execute blocks of transactions and measure the timr taken for \
                each concurrency level."
    )]
    num_repeats: usize,

    #[clap(
        long,
        help = "If true, measure time taken to execute each block separately. If false, measure \
                the overall time to execute all blocks."
    )]
    measure_block_times: bool,

    #[clap(
        long,
        num_args = 1..,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to enable, in capital letters. For example, \
                GAS_PAYER_ENABLED or EMIT_FEE_STATEMENT. For the full list of feature flags, see \
                aptos-core/types/src/on_chain_config/aptos_features.rs."
    )]
    enable_features: Vec<FeatureFlag>,

    #[clap(
        long,
        num_args = 1..,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to disable, in capital letters. For \
                example, GAS_PAYER_ENABLED or EMIT_FEE_STATEMENT. For the full list of feature \
                flags, see aptos-core/types/src/on_chain_config/aptos_features.rs."
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
        "Transaction versions should be a valid closed interval. Instead got begin: {}, end: {}",
        command.begin_version,
        command.end_version,
    );
    assert!(
        !command.concurrency_levels.is_empty(),
        "At least one concurrency level must be provided",
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
    let builder = Client::builder(AptosBaseUrl::Custom(Url::parse(&command.rest_endpoint)?));
    let client = if let Some(api_key) = command.api_key {
        builder.api_key(&api_key)?.build()
    } else {
        builder.build()
    };
    let debugger = AptosDebugger::rest_client(client)?;

    // TODO:
    //  Right now, only features can be overridden. In general, this can be allowed for anything:
    //      1. Framework code, e.g., to test performance of new natives or compiler,
    //      2. Gas schedule, to track the costs of charging gas or tracking limits.
    //  We probably should support at least these.
    let override_config = OverrideConfig::new(command.enable_features, command.disable_features);

    let blocks = BenchmarkGenerator::generate_blocks(
        debugger,
        command.begin_version,
        command.end_version,
        override_config,
    )
    .await?;

    // Ensure we have at least one block to benchmark.
    assert!(
        command.num_blocks_to_skip < blocks.len(),
        "There are only {} blocks, but skipping {}",
        blocks.len(),
        command.num_blocks_to_skip
    );

    for block in &blocks {
        block.print_diffs();
    }

    BenchmarkRunner::new(
        command.concurrency_levels,
        command.num_repeats,
        command.measure_block_times,
        command.num_blocks_to_skip,
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
