// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::init_logger_and_metrics,
    runner::{BenchmarkRunner, Block},
    state_view::ReadSet,
    workload::TransactionBlock,
};
use anyhow::anyhow;
use aptos_logger::Level;
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

/// Minimum number of times to execute blocks of transactions and measure the time taken.
const MIN_NUM_REPEATS: usize = 3;

#[derive(Parser)]
#[command(about = "Executes saved transactions on top of the saved state")]
pub struct BenchmarkCommand {
    #[clap(long, default_value_t = Level::Error)]
    log_level: Level,

    #[clap(long, help = "Directory where transactions are saved")]
    transactions_file: String,

    #[clap(long, help = "Directory where the state is be saved")]
    inputs_file: String,

    #[clap(
        long,
        default_value_t = 0,
        help = "Number of blocks to skip for time measurement. Allows to warm-up caches"
    )]
    num_blocks_to_skip: usize,

    #[clap(
        long,
        num_args = 1..,
        help = "List of space-separated concurrency levels that define how many threads Block-STM \
                is using to execute a block of transactions. For each level, the time taken to \
                execute blocks of transactions is measured and reported"
    )]
    concurrency_levels: Vec<usize>,

    #[clap(
        long,
        default_value_t = MIN_NUM_REPEATS,
        help = "Number of times to execute blocks of transactions and measure the timr taken for \
                each concurrency level"
    )]
    num_repeats: usize,

    #[clap(
        long,
        help = "If true, measure time taken to execute each block separately. If false, measure \
                the overall time to execute all blocks"
    )]
    measure_block_times: bool,
}

impl BenchmarkCommand {
    pub async fn benchmark(self) -> anyhow::Result<()> {
        init_logger_and_metrics(self.log_level);

        // Sanity checks for provided commands.
        assert!(
            !self.concurrency_levels.is_empty(),
            "At least one concurrency level must be provided",
        );
        assert!(
            self.num_repeats >= MIN_NUM_REPEATS,
            "Number of repeats must be at least {}",
            MIN_NUM_REPEATS,
        );

        let txn_blocks_bytes = fs::read(PathBuf::from(&self.transactions_file)).await?;
        let txn_blocks: Vec<TransactionBlock> =
            bcs::from_bytes(&txn_blocks_bytes).map_err(|err| {
                anyhow!(
                    "Error when deserializing a block of transactions: {:?}",
                    err
                )
            })?;

        let input_bytes = fs::read(PathBuf::from(&self.inputs_file)).await?;
        let inputs: Vec<ReadSet> = bcs::from_bytes(&input_bytes)
            .map_err(|err| anyhow!("Error when deserializing inputs: {:?}", err))?;

        // Ensure we have at least one block to benchmark.
        assert_eq!(txn_blocks.len(), inputs.len());
        assert!(
            self.num_blocks_to_skip < txn_blocks.len(),
            "There are only {} blocks, but skipping {}",
            txn_blocks.len(),
            self.num_blocks_to_skip
        );

        let blocks = inputs
            .into_iter()
            .zip(txn_blocks)
            .map(|(inputs, txn_block)| Block {
                inputs,
                workload: txn_block.into(),
            })
            .collect::<Vec<_>>();

        BenchmarkRunner::new(
            self.concurrency_levels,
            self.num_repeats,
            self.measure_block_times,
            self.num_blocks_to_skip,
        )
        .measure_execution_time(&blocks);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        BenchmarkCommand::command().debug_assert();
    }
}
