// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{
        chunk_output::{update_counters_for_processed_chunk, ChunkOutput},
        in_memory_state_calculator_v2::InMemoryStateCalculatorV2,
    },
    metrics::{APTOS_EXECUTOR_ERRORS, APTOS_EXECUTOR_OTHER_TIMERS_SECONDS},
};
use anyhow::{ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{
    parsed_transaction_output::TransactionsWithParsedOutput,
    should_forward_to_subscription_service,
    state_checkpoint_output::{StateCheckpointOutput, TransactionsByStatus},
    ExecutedChunk, LedgerUpdateOutput, ParsedTransactionOutput,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::error;
use aptos_storage_interface::{state_delta::StateDelta, ExecutedTrees};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::accumulator::{InMemoryEventAccumulator, InMemoryTransactionAccumulator},
    state_store::ShardedStateUpdates,
    transaction::{
        block_epilogue::{BlockEndInfo, BlockEpiloguePayload},
        ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionOutput,
        TransactionStatus, TransactionToCommit,
    },
    write_set::WriteSet,
};
use rayon::prelude::*;
use std::{iter::repeat, sync::Arc};

pub struct ApplyChunkOutput;

impl ApplyChunkOutput {
    pub fn calculate_state_checkpoint(
        chunk_output: ChunkOutput,
        parent_state: &StateDelta,
        append_state_checkpoint_to_block: Option<HashValue>,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        is_block: bool,
    ) -> Result<(StateDelta, Option<EpochState>, StateCheckpointOutput)> {
        let ChunkOutput {
            state_cache,
            transactions,
            transaction_outputs,
            block_end_info,
        } = chunk_output;
        let (new_epoch, statuses_for_input_txns, to_commit, to_discard, to_retry) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["sort_transactions"])
                .start_timer();
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
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
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

        Ok((result_state, next_epoch_state, state_checkpoint_output))
    }

    pub fn calculate_ledger_update(
        state_checkpoint_output: StateCheckpointOutput,
        base_txn_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Result<(LedgerUpdateOutput, Vec<Transaction>, Vec<Transaction>)> {
        let (
            txns,
            state_updates_vec,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
            block_end_info,
        ) = state_checkpoint_output.into_inner();

        let (statuses_for_input_txns, to_commit, to_discard, to_retry) = txns.into_inner();

        update_counters_for_processed_chunk(
            to_commit.txns(),
            to_commit.parsed_outputs(),
            "execution",
        );
        update_counters_for_processed_chunk(
            to_discard.txns(),
            to_discard.parsed_outputs(),
            "execution",
        );
        update_counters_for_processed_chunk(
            to_retry.txns(),
            to_retry.parsed_outputs(),
            "execution",
        );

        // Calculate TransactionData and TransactionInfo, i.e. the ledger history diff.
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["assemble_ledger_diff_for_block"])
            .start_timer();

        let (txns_to_commit, transaction_info_hashes, subscribable_events) =
            Self::assemble_ledger_diff(to_commit, state_updates_vec, state_checkpoint_hashes);
        let transaction_accumulator =
            Arc::new(base_txn_accumulator.append(&transaction_info_hashes));
        Ok((
            LedgerUpdateOutput {
                statuses_for_input_txns,
                to_commit: txns_to_commit,
                subscribable_events,
                transaction_info_hashes,
                state_updates_until_last_checkpoint: state_updates_before_last_checkpoint,
                sharded_state_cache,
                transaction_accumulator,
                block_end_info,
            },
            to_discard.into_txns(),
            to_retry.into_txns(),
        ))
    }

    pub fn apply_chunk(
        chunk_output: ChunkOutput,
        base_view: &ExecutedTrees,
        known_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        let (result_state, next_epoch_state, state_checkpoint_output) =
            Self::calculate_state_checkpoint(
                chunk_output,
                base_view.state(),
                None, // append_state_checkpoint_to_block
                known_state_checkpoint_hashes,
                /*is_block=*/ false,
            )?;
        let (ledger_update_output, to_discard, to_retry) = Self::calculate_ledger_update(
            state_checkpoint_output,
            base_view.txn_accumulator().clone(),
        )?;

        Ok((
            ExecutedChunk {
                result_state,
                ledger_info: None,
                next_epoch_state,
                ledger_update_output,
            },
            to_discard,
            to_retry,
        ))
    }

    fn sort_transactions_with_state_checkpoint(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        append_state_checkpoint_to_block: Option<HashValue>,
        block_end_info: Option<BlockEndInfo>,
    ) -> Result<(
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
            TransactionsWithParsedOutput::new(vec![], vec![])
        };

        let state_checkpoint_to_add =
            new_epoch_marker.map_or_else(|| append_state_checkpoint_to_block, |_| None);

        let keeps_and_discards = transaction_outputs.iter().map(|t| t.status()).cloned();
        let retries = repeat(TransactionStatus::Retry).take(to_retry.len());

        let status = keeps_and_discards.chain(retries).collect();

        // Separate transactions with the Keep status out.
        let (mut to_keep, to_discard) = itertools::zip_eq(transactions, transaction_outputs)
            .partition::<Vec<(Transaction, ParsedTransactionOutput)>, _>(|(_, o)| {
                matches!(o.status(), TransactionStatus::Keep(_))
            });

        // Append the StateCheckpoint transaction to the end of to_keep
        if let Some(block_id) = state_checkpoint_to_add {
            let state_checkpoint_txn =
                block_end_info.map_or(Transaction::StateCheckpoint(block_id), |block_end_info| {
                    Transaction::BlockEpilogue(BlockEpiloguePayload::V0 {
                        block_id,
                        block_end_info,
                    })
                });
            let state_checkpoint_txn_output: ParsedTransactionOutput =
                Into::into(TransactionOutput::new(
                    WriteSet::default(),
                    Vec::new(),
                    0,
                    TransactionStatus::Keep(ExecutionStatus::Success),
                    TransactionAuxiliaryData::default(),
                ));
            to_keep.push((state_checkpoint_txn, state_checkpoint_txn_output));
        }

        // Sanity check transactions with the Discard status:
        let to_discard = to_discard
            .into_iter()
            .map(|(t, o)| {
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
                    APTOS_EXECUTOR_ERRORS.inc();
                }
                Ok((t, o))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            new_epoch_marker.is_some(),
            status,
            to_keep.into(),
            to_discard.into(),
            to_retry,
        ))
    }

    fn assemble_ledger_diff(
        to_commit_from_execution: TransactionsWithParsedOutput,
        state_updates_vec: Vec<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> (Vec<TransactionToCommit>, Vec<HashValue>, Vec<ContractEvent>) {
        // these are guaranteed by caller side logic
        assert_eq!(to_commit_from_execution.len(), state_updates_vec.len());
        assert_eq!(
            to_commit_from_execution.len(),
            state_checkpoint_hashes.len()
        );

        let num_txns = to_commit_from_execution.len();
        let mut to_commit = Vec::with_capacity(num_txns);
        let mut txn_info_hashes = Vec::with_capacity(num_txns);
        let hashes_vec =
            Self::calculate_events_and_writeset_hashes(to_commit_from_execution.parsed_outputs());
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["process_events_and_writeset_hashes"])
            .start_timer();
        let hashes_vec: Vec<(HashValue, HashValue)> = hashes_vec
            .into_par_iter()
            .map(|(event_hashes, write_set_hash)| {
                (
                    InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash(),
                    write_set_hash,
                )
            })
            .collect();

        let mut all_subscribable_events = Vec::new();
        let (to_commit_txns, to_commit_outputs) = to_commit_from_execution.into_inner();
        for (
            txn,
            txn_output,
            state_checkpoint_hash,
            state_updates,
            (event_root_hash, write_set_hash),
        ) in itertools::izip!(
            to_commit_txns,
            to_commit_outputs,
            state_checkpoint_hashes,
            state_updates_vec,
            hashes_vec
        ) {
            let (write_set, events, per_txn_reconfig_events, gas_used, status, auxiliary_data) =
                txn_output.unpack();

            let subscribable_events: Vec<ContractEvent> = events
                .iter()
                .filter(|evt| should_forward_to_subscription_service(evt))
                .cloned()
                .collect();
            let txn_info = match &status {
                TransactionStatus::Keep(status) => TransactionInfo::new(
                    txn.hash(),
                    write_set_hash,
                    event_root_hash,
                    state_checkpoint_hash,
                    gas_used,
                    status.clone(),
                ),
                _ => unreachable!("Transaction sorted by status already."),
            };
            let txn_info_hash = txn_info.hash();
            txn_info_hashes.push(txn_info_hash);
            let txn_to_commit = TransactionToCommit::new(
                txn,
                txn_info,
                state_updates,
                write_set,
                events,
                !per_txn_reconfig_events.is_empty(),
                auxiliary_data,
            );
            all_subscribable_events.extend(subscribable_events);
            to_commit.push(txn_to_commit);
        }
        (to_commit, txn_info_hashes, all_subscribable_events)
    }

    fn calculate_events_and_writeset_hashes(
        to_commit_from_execution: &[ParsedTransactionOutput],
    ) -> Vec<(Vec<HashValue>, HashValue)> {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_events_and_writeset_hashes"])
            .start_timer();
        let num_txns = to_commit_from_execution.len();
        to_commit_from_execution
            .par_iter()
            .with_min_len(optimal_min_len(num_txns, 64))
            .map(|txn_output| {
                (
                    txn_output
                        .events()
                        .iter()
                        .map(CryptoHash::hash)
                        .collect::<Vec<_>>(),
                    CryptoHash::hash(txn_output.write_set()),
                )
            })
            .collect::<Vec<_>>()
    }
}

pub fn ensure_no_discard(to_discard: Vec<Transaction>) -> Result<()> {
    ensure!(to_discard.is_empty(), "Syncing discarded transactions");
    Ok(())
}

pub fn ensure_no_retry(to_retry: Vec<Transaction>) -> Result<()> {
    ensure!(
        to_retry.is_empty(),
        "Seeing retries when syncing, did it crosses epoch boundary?",
    );
    Ok(())
}

#[test]
fn assemble_ledger_diff_should_filter_subscribable_events() {
    let event_0 =
        ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_1".to_vec());
    let event_1 = ContractEvent::new_v2_with_type_tag_str(
        "0x2345::random_module::RandomEvent",
        b"random_x".to_vec(),
    );
    let event_2 =
        ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_2".to_vec());
    let txns_n_outputs =
        TransactionsWithParsedOutput::new(vec![Transaction::dummy(), Transaction::dummy()], vec![
            ParsedTransactionOutput::from(TransactionOutput::new(
                WriteSet::default(),
                vec![event_0.clone()],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            )),
            ParsedTransactionOutput::from(TransactionOutput::new(
                WriteSet::default(),
                vec![event_1.clone(), event_2.clone()],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            )),
        ]);
    let state_updates_vec = vec![
        ShardedStateUpdates::default(),
        ShardedStateUpdates::default(),
    ];
    let state_checkpoint_hashes = vec![Some(HashValue::zero()), Some(HashValue::zero())];
    let (_, _, subscribable_events) = ApplyChunkOutput::assemble_ledger_diff(
        txns_n_outputs,
        state_updates_vec,
        state_checkpoint_hashes,
    );
    assert_eq!(vec![event_0, event_2], subscribable_events);
}
