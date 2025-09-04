// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::init_logger_and_metrics,
    diff::TransactionDiffBuilder,
    execution::execute_workload,
    state_view::ReadSet,
    workload::{TransactionBlock, Workload},
};
use anyhow::{anyhow, bail};
use velor_logger::Level;
use velor_types::transaction::TransactionOutput;
use velor_vm::{velor_vm::VelorVMBlockExecutor, VMBlockExecutor};
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

#[derive(Parser)]
#[command(about = "Compares execution outputs for transactions executed on different states")]
pub struct DiffCommand {
    #[clap(long, default_value_t = Level::Error)]
    log_level: Level,

    #[clap(long, help = "File where the transactions are saved")]
    transactions_file: String,

    #[clap(long, help = "File where the input states are saved")]
    inputs_file: String,

    #[clap(long, help = "File where the other input states are saved")]
    other_inputs_file: String,

    #[clap(
        long,
        default_value_t = 1,
        help = "Concurrency level for block execution"
    )]
    concurrency_level: usize,

    #[clap(
        long,
        default_value_t = false,
        help = "If true, when comparing output diffs changes related to gas usage are ignored"
    )]
    allow_different_gas_usage: bool,
}

impl DiffCommand {
    pub async fn diff_outputs(self) -> anyhow::Result<()> {
        init_logger_and_metrics(self.log_level);

        let txn_blocks_bytes = fs::read(PathBuf::from(&self.transactions_file)).await?;
        let txn_blocks: Vec<TransactionBlock> = bcs::from_bytes(&txn_blocks_bytes)
            .map_err(|err| anyhow!("Error when deserializing blocks of transactions: {:?}", err))?;
        if txn_blocks.is_empty() {
            bail!("There must be at least one transaction to execute");
        }

        let inputs_read_set_bytes = fs::read(PathBuf::from(&self.inputs_file)).await?;
        let inputs_read_set: Vec<ReadSet> = bcs::from_bytes(&inputs_read_set_bytes)
            .map_err(|err| anyhow!("Error when deserializing inputs: {:?}", err))?;

        let other_inputs_read_set_bytes = fs::read(PathBuf::from(&self.other_inputs_file)).await?;
        let other_inputs_read_set: Vec<ReadSet> = bcs::from_bytes(&other_inputs_read_set_bytes)
            .map_err(|err| anyhow!("Error when deserializing other inputs: {:?}", err))?;

        // For later comparison of outputs, find fee payers for transactions.
        let fee_payers = txn_blocks
            .iter()
            .map(|txn_block| {
                txn_block
                    .transactions
                    .iter()
                    .map(|txn| {
                        txn.try_as_signed_user_txn().map(|txn| {
                            txn.authenticator_ref()
                                .fee_payer_address()
                                .unwrap_or_else(|| txn.sender())
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let workloads = txn_blocks
            .into_iter()
            .map(Workload::from)
            .collect::<Vec<_>>();

        // Ensure the number of blocks matches.
        if workloads.len() != inputs_read_set.len()
            || inputs_read_set.len() != other_inputs_read_set.len()
        {
            bail!(
                "Number of blocks of transactions does not match the number of pre-block states: \
                there {} blocks, but {} and {} input states",
                workloads.len(),
                inputs_read_set.len(),
                other_inputs_read_set.len()
            );
        }

        let outputs = self.compute_outputs(&workloads, &inputs_read_set);
        let other_outputs = self.compute_outputs(&workloads, &other_inputs_read_set);

        let mut version = workloads[0]
            .transaction_slice_metadata
            .begin_version()
            .expect("Begin version must be set");

        let diff_builder = TransactionDiffBuilder::new(self.allow_different_gas_usage);
        let mut diffs = Vec::with_capacity(outputs.len());

        println!(
            "block, {} (gas), {} (gas)",
            self.inputs_file, self.other_inputs_file
        );
        for (idx, ((outputs, other_outputs), fee_payers)) in outputs
            .into_iter()
            .zip(other_outputs)
            .zip(fee_payers)
            .enumerate()
        {
            let mut block_gas_used = 0;
            let mut other_block_gas_used = 0;

            for ((output, other_output), fee_payer) in
                outputs.into_iter().zip(other_outputs).zip(fee_payers)
            {
                block_gas_used += output.gas_used();
                other_block_gas_used += other_output.gas_used();

                let diff = diff_builder.build_from_outputs(output, other_output, fee_payer);
                if !diff.is_empty() {
                    diffs.push((version, diff));
                }
                version += 1;
            }
            println!("{}, {}, {}", idx + 1, block_gas_used, other_block_gas_used);
        }

        for (version, diff) in diffs {
            println!("Non-empty output diff for transaction {}:", version);
            diff.println();
        }

        Ok(())
    }

    /// Returns outputs for the specified blocks of transactions and inputs.
    fn compute_outputs(
        &self,
        workloads: &[Workload],
        inputs: &[ReadSet],
    ) -> Vec<Vec<TransactionOutput>> {
        let executor = VelorVMBlockExecutor::new();
        workloads
            .iter()
            .zip(inputs)
            .map(|(workload, input)| {
                execute_workload(&executor, workload, input, self.concurrency_level)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        DiffCommand::command().debug_assert();
    }
}
