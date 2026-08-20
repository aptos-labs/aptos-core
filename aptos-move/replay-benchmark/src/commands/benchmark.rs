// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    commands::init_logger_and_metrics,
    runner::{BenchmarkRunner, ReplayBlock},
    state_view::ReadSet,
    workload::retain_user_transactions_only,
};
use anyhow::{anyhow, bail};
use aptos_logger::Level;
use aptos_types::transaction::TransactionBlock;
use aptos_vm_environment::prod_configs::{
    set_async_runtime_checks, set_layout_caches, set_paranoid_type_checks,
};
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

/// Minimum number of times to execute blocks of transactions and measure the time taken.
const MIN_NUM_REPEATS: usize = 3;

#[derive(Parser)]
#[command(
    about = "Executes saved transactions on top of the saved state, and reports the time taken"
)]
pub struct BenchmarkCommand {
    #[clap(long, default_value_t = Level::Error)]
    log_level: Level,

    #[clap(long, help = "File where blocks of transactions are saved")]
    transactions_file: String,

    #[clap(long, help = "File where the input states are saved")]
    inputs_file: String,

    #[clap(
        long,
        default_value_t = 0,
        help = "Number of blocks to skip for time measurement. Allows to warm-up caches"
    )]
    num_blocks_to_skip: usize,

    #[clap(
        long,
        default_value_t = 0,
        help = "Number of untimed warm-up passes over all blocks before timing. A single executor \
                is reused across warm-up and every measured repeat, so the module and code caches \
                populated during warm-up stay warm when timing. Use 1 to benchmark a single block \
                warm, e.g. for MonoMove whose global context is built cold per executor"
    )]
    num_warmups: usize,

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
        help = "Number of times to execute blocks of transactions and measure the time taken for \
                each concurrency level. Should be at least 3."
    )]
    num_repeats: usize,

    #[clap(
        long,
        default_value_t = false,
        help = "If false, measure time taken to execute each block separately. If true, measure \
                the overall time to execute all blocks"
    )]
    measure_overall_time: bool,

    #[clap(
        long,
        default_value_t = false,
        help = "If false, Move VM runs in paranoid mode, if true, paranoid mode is not used"
    )]
    disable_paranoid_mode: bool,

    #[clap(
        long,
        default_value_t = false,
        help = "If true, Move VM runs traces execution, and Block-STM performs checks later"
    )]
    async_runtime_checks: bool,

    #[clap(
        long,
        default_value_t = false,
        help = "If set, keep only signed user transactions in each block, dropping block \
                metadata, state checkpoint, block epilogue, validator, and genesis transactions \
                (and dropping blocks left empty). Must match the value passed to `initialize` so \
                the saved input states line up with the replayed workload. Useful for VMs that \
                execute only user transactions, e.g. MonoMove"
    )]
    user_txns_only: bool,
}

impl BenchmarkCommand {
    pub async fn benchmark(self) -> anyhow::Result<()> {
        init_logger_and_metrics(self.log_level);

        // Sanity checks for provided commands.
        if self.concurrency_levels.is_empty() {
            bail!("At least one concurrency level must be provided");
        }
        if self.num_repeats < MIN_NUM_REPEATS {
            bail!("Number of repeats must be at least {}", MIN_NUM_REPEATS,);
        }

        let txn_blocks_bytes = fs::read(PathBuf::from(&self.transactions_file)).await?;
        let txn_blocks: Vec<TransactionBlock> = bcs::from_bytes(&txn_blocks_bytes)
            .map_err(|err| anyhow!("Error when deserializing blocks of transactions: {:?}", err))?;
        let txn_blocks = if self.user_txns_only {
            retain_user_transactions_only(txn_blocks)
        } else {
            txn_blocks
        };

        let inputs_read_set_bytes = fs::read(PathBuf::from(&self.inputs_file)).await?;
        let inputs_read_set: Vec<ReadSet> = bcs::from_bytes(&inputs_read_set_bytes)
            .map_err(|err| anyhow!("Error when deserializing inputs: {:?}", err))?;

        // Ensure we have at least one block to benchmark, and that we do not skip all the blocks.
        if txn_blocks.len() != inputs_read_set.len() {
            bail!(
                "Number of transaction blocks does not match the number of pre-block input states: \
                there are {} blocks and {} inputs",
                txn_blocks.len(),
                inputs_read_set.len()
            )
        }
        if self.num_blocks_to_skip >= txn_blocks.len() {
            bail!(
                "There are only {} blocks, but skipping {}",
                txn_blocks.len(),
                self.num_blocks_to_skip
            );
        }

        let blocks = inputs_read_set
            .into_iter()
            .zip(txn_blocks)
            .map(|(inputs, txn_block)| ReplayBlock {
                inputs,
                workload: txn_block.into(),
            })
            .collect::<Vec<_>>();

        set_layout_caches(true);
        set_paranoid_type_checks(!self.disable_paranoid_mode);
        set_async_runtime_checks(self.async_runtime_checks);
        BenchmarkRunner::new(
            self.concurrency_levels,
            self.num_repeats,
            self.measure_overall_time,
            self.num_blocks_to_skip,
            self.num_warmups,
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
