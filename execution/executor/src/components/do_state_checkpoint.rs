// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    components::in_memory_state_calculator_v2::InMemoryStateCalculatorV2,
    metrics::{EXECUTOR_ERRORS, OTHER_TIMERS},
};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput,
    parsed_transaction_output::TransactionsWithParsedOutput,
    state_checkpoint_output::{StateCheckpointOutput, TransactionsByStatus},
    ParsedTransactionOutput,
};
use aptos_logger::error;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_delta::StateDelta;
use aptos_types::{
    epoch_state::EpochState,
    transaction::{
        BlockEndInfo, BlockEpiloguePayload, ExecutionStatus, Transaction, TransactionAuxiliaryData,
        TransactionOutput, TransactionStatus,
    },
    write_set::WriteSet,
};
use std::{iter::repeat, sync::Arc};

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        chunk_output: ExecutionOutput,
        parent_state: &StateDelta,
        append_state_checkpoint_to_block: Option<HashValue>,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        is_block: bool,
    ) -> anyhow::Result<(Arc<StateDelta>, Option<EpochState>, StateCheckpointOutput)> {
        let ExecutionOutput {
            state_cache,
            transactions,
            transaction_outputs,
            block_end_info,
        } = chunk_output;
        let (new_epoch, statuses_for_input_txns, to_commit, to_discard, to_retry) = {
            let _timer = OTHER_TIMERS.timer_with(&["sort_transactions"]);

            // Separate transactions with different VM statuses, i.e., Keep, Discard and Retry.
            // Will return transactions with Retry txns sorted after Keep/Discard txns.
            Self::sort_transactions_with_state_checkpoint(
                transactions,
                transaction_outputs,
                append_state_checkpoint_to_block,
                block_end_info.clone(),
            )?
        };

        // Apply the write set, get the latest state.
        let (
            state_updates_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
        ) = {
            let _timer = OTHER_TIMERS
                .with_label_values(&["calculate_for_transactions"])
                .start_timer();
            InMemoryStateCalculatorV2::calculate_for_transactions(
                parent_state,
                state_cache,
                &to_commit,
                new_epoch,
                is_block,
            )?
        };

        let mut state_checkpoint_output = StateCheckpointOutput::new(
            TransactionsByStatus::new(statuses_for_input_txns, to_commit, to_discard, to_retry),
            state_updates_vec,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
            block_end_info,
        );

        // On state sync/replay, we generate state checkpoints only periodically, for the
        // last state checkpoint of each chunk.
        // A mismatch in the SMT will be detected at that occasion too. Here we just copy
        // in the state root from the TxnInfo in the proof.
        if let Some(state_checkpoint_hashes) = known_state_checkpoints {
            state_checkpoint_output
                .check_and_update_state_checkpoint_hashes(state_checkpoint_hashes)?;
        }

        Ok((
            Arc::new(result_state),
            next_epoch_state,
            state_checkpoint_output,
        ))
    }

    fn sort_transactions_with_state_checkpoint(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        append_state_checkpoint_to_block: Option<HashValue>,
        block_end_info: Option<BlockEndInfo>,
    ) -> anyhow::Result<(
        bool,
        Vec<TransactionStatus>,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
    )> {
        let mut transaction_outputs: Vec<ParsedTransactionOutput> =
            transaction_outputs.into_iter().map(Into::into).collect();
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        let block_gas_limit_marker = transaction_outputs
            .iter()
            .position(|o| matches!(o.status(), TransactionStatus::Retry));

        // Transactions after the epoch ending txn are all to be retried.
        // Transactions after the txn that exceeded per-block gas limit are also to be retried.
        let to_retry = if let Some(pos) = new_epoch_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else if let Some(pos) = block_gas_limit_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else {
            TransactionsWithParsedOutput::new_empty()
        };

        let state_checkpoint_to_add =
            new_epoch_marker.map_or_else(|| append_state_checkpoint_to_block, |_| None);

        let keeps_and_discards = transaction_outputs.iter().map(|t| t.status()).cloned();
        let retries = repeat(TransactionStatus::Retry).take(to_retry.len());

        let status = keeps_and_discards.chain(retries).collect();

        let to_discard = {
            let mut res = TransactionsWithParsedOutput::new_empty();
            for idx in 0..transactions.len() {
                if transaction_outputs[idx].status().is_discarded() {
                    res.push(transactions[idx].clone(), transaction_outputs[idx].clone());
                } else if !res.is_empty() {
                    transactions[idx - res.len()] = transactions[idx].clone();
                    transaction_outputs[idx - res.len()] = transaction_outputs[idx].clone();
                }
            }
            if !res.is_empty() {
                let remaining = transactions.len() - res.len();
                transactions.truncate(remaining);
                transaction_outputs.truncate(remaining);
            }
            res
        };
        let to_keep = {
            let mut res = TransactionsWithParsedOutput::new(transactions, transaction_outputs);

            // Append the StateCheckpoint transaction to the end of to_keep
            if let Some(block_id) = state_checkpoint_to_add {
                let state_checkpoint_txn = block_end_info.map_or(
                    Transaction::StateCheckpoint(block_id),
                    |block_end_info| {
                        Transaction::BlockEpilogue(BlockEpiloguePayload::V0 {
                            block_id,
                            block_end_info,
                        })
                    },
                );
                let state_checkpoint_txn_output: ParsedTransactionOutput =
                    Into::into(TransactionOutput::new(
                        WriteSet::default(),
                        Vec::new(),
                        0,
                        TransactionStatus::Keep(ExecutionStatus::Success),
                        TransactionAuxiliaryData::default(),
                    ));
                res.push(state_checkpoint_txn, state_checkpoint_txn_output);
            }
            res
        };

        // Sanity check transactions with the Discard status:
        to_discard.iter().for_each(|(t, o)| {
            // In case a new status other than Retry, Keep and Discard is added:
            if !matches!(o.status(), TransactionStatus::Discard(_)) {
                error!("Status other than Retry, Keep or Discard; Transaction discarded.");
            }
            // VM shouldn't have output anything for discarded transactions, log if it did.
            if !o.write_set().is_empty() || !o.events().is_empty() {
                error!(
                    "Discarded transaction has non-empty write set or events. \
                        Transaction: {:?}. Status: {:?}.",
                    t,
                    o.status(),
                );
                EXECUTOR_ERRORS.inc();
            }
        });

        Ok((
            new_epoch_marker.is_some(),
            status,
            to_keep,
            to_discard,
            to_retry,
        ))
    }
}
