// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{components::chunk_output::ChunkOutput, metrics::APTOS_EXECUTOR_ERRORS};
use anyhow::{ensure, Result};
use aptos_crypto::{
    hash::{CryptoHash, EventAccumulatorHasher},
    HashValue,
};
use aptos_logger::error;
use aptos_types::{
    proof::accumulator::InMemoryAccumulator,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, TransactionOutput, TransactionStatus},
};
use executor_types::{
    in_memory_state_calculator::InMemoryStateCalculator, ExecutedChunk, ParsedTransactionOutput,
    TransactionData,
};
use std::{collections::HashMap, iter::repeat, sync::Arc};
use storage_interface::ExecutedTrees;

pub struct ApplyChunkOutput;

impl ApplyChunkOutput {
    pub fn apply(
        chunk_output: ChunkOutput,
        base_view: &ExecutedTrees,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        let ChunkOutput {
            state_cache,
            transactions,
            transaction_outputs,
        } = chunk_output;
        // Separate transactions with different VM statuses.
        let (new_epoch, status, to_keep, to_discard, to_retry) =
            Self::sort_transactions(transactions, transaction_outputs)?;

        // Apply the write set, get the latest state.
        let (state_updates_vec, state_checkpoint_hashes, result_state, next_epoch_state) =
            InMemoryStateCalculator::new(base_view.state(), state_cache)
                .calculate_for_transaction_chunk(&to_keep, new_epoch)?;

        // Calculate TransactionData and TransactionInfo, i.e. the ledger history diff.
        let (to_commit, transaction_info_hashes) =
            Self::assemble_ledger_diff(to_keep, state_updates_vec, state_checkpoint_hashes);
        let result_view = ExecutedTrees::new(
            result_state,
            Arc::new(base_view.txn_accumulator().append(&transaction_info_hashes)),
        );

        Ok((
            ExecutedChunk {
                status,
                to_commit,
                result_view,
                next_epoch_state,
                ledger_info: None,
            },
            to_discard,
            to_retry,
        ))
    }

    fn sort_transactions(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
    ) -> Result<(
        bool,
        Vec<TransactionStatus>,
        Vec<(Transaction, ParsedTransactionOutput)>,
        Vec<Transaction>,
        Vec<Transaction>,
    )> {
        let num_txns = transactions.len();
        let mut transaction_outputs: Vec<ParsedTransactionOutput> =
            transaction_outputs.into_iter().map(Into::into).collect();
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        // Transactions after the epoch ending are all to be retried.
        let to_retry = if let Some(pos) = new_epoch_marker {
            transaction_outputs.drain(pos..);
            transactions.drain(pos..).collect()
        } else {
            vec![]
        };

        // N.B. Transaction status after the epoch marker are ignored and set to Retry forcibly.
        let status = transaction_outputs
            .iter()
            .map(|t| t.status())
            .cloned()
            .chain(repeat(TransactionStatus::Retry))
            .take(num_txns)
            .collect();

        // Separate transactions with the Keep status out.
        let (to_keep, to_discard) =
            itertools::zip_eq(transactions.into_iter(), transaction_outputs.into_iter())
                .partition::<Vec<(Transaction, ParsedTransactionOutput)>, _>(|(_, o)| {
                    matches!(o.status(), TransactionStatus::Keep(_))
                });

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
                Ok(t)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            new_epoch_marker.is_some(),
            status,
            to_keep,
            to_discard,
            to_retry,
        ))
    }

    fn assemble_ledger_diff(
        to_keep: Vec<(Transaction, ParsedTransactionOutput)>,
        state_updates_vec: Vec<HashMap<StateKey, Option<StateValue>>>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> (Vec<(Transaction, TransactionData)>, Vec<HashValue>) {
        // these are guaranteed by caller side logic
        assert_eq!(to_keep.len(), state_updates_vec.len());
        assert_eq!(to_keep.len(), state_checkpoint_hashes.len());

        let mut to_commit = vec![];
        let mut txn_info_hashes = vec![];
        for (((txn, txn_output), state_checkpoint_hash), state_updates) in itertools::zip_eq(
            itertools::zip_eq(to_keep, state_checkpoint_hashes),
            state_updates_vec,
        ) {
            let (write_set, events, reconfig_events, gas_used, status) = txn_output.unpack();
            let event_tree = {
                let event_hashes: Vec<_> = events.iter().map(CryptoHash::hash).collect();
                InMemoryAccumulator::<EventAccumulatorHasher>::from_leaves(&event_hashes)
            };

            let state_change_hash = CryptoHash::hash(&write_set);
            let txn_info = match &status {
                TransactionStatus::Keep(status) => TransactionInfo::new(
                    txn.hash(),
                    state_change_hash,
                    event_tree.root_hash(),
                    state_checkpoint_hash,
                    gas_used,
                    status.clone(),
                ),
                _ => unreachable!("Transaction sorted by status already."),
            };
            let txn_info_hash = txn_info.hash();
            txn_info_hashes.push(txn_info_hash);
            to_commit.push((
                txn,
                TransactionData::new(
                    state_updates,
                    write_set,
                    events,
                    reconfig_events,
                    status,
                    Arc::new(event_tree),
                    gas_used,
                    txn_info,
                    txn_info_hash,
                ),
            ))
        }
        (to_commit, txn_info_hashes)
    }
}

pub fn ensure_no_discard(to_discard: Vec<Transaction>) -> Result<()> {
    ensure!(to_discard.is_empty(), "Syncing discarded transactions");
    Ok(())
}

pub fn ensure_no_retry(to_retry: Vec<Transaction>) -> Result<()> {
    ensure!(to_retry.is_empty(), "Chunk crosses epoch boundary.",);
    Ok(())
}
