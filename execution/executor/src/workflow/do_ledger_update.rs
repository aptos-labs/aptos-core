// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::OTHER_TIMERS;
use anyhow::Result;
use aptos_crypto::{HashValue, hash::CryptoHash};
use aptos_executor_types::{
    LedgerUpdateOutput, execution_output::ExecutionOutput,
    state_checkpoint_output::StateCheckpointOutput,
    transactions_with_output::TransactionsWithOutput,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    proof::accumulator::{InMemoryEventAccumulator, InMemoryTransactionAccumulator},
    transaction::{PersistedAuxiliaryInfo, TransactionInfo, TransactionOutput},
};
use itertools::{Itertools, izip};
use rayon::prelude::*;
use std::sync::Arc;

pub struct DoLedgerUpdate;

impl DoLedgerUpdate {
    pub fn run(
        execution_output: &ExecutionOutput,
        state_checkpoint_output: &StateCheckpointOutput,
        parent_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Result<LedgerUpdateOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["do_ledger_update"]);

        // Calculate hashes
        let txn_outs = &execution_output.to_commit.transaction_outputs;

        let (event_hashes, writeset_hashes) = Self::calculate_events_and_writeset_hashes(txn_outs);

        // Assemble `TransactionInfo`s
        let transaction_infos = Self::assemble_transaction_infos(
            &execution_output.to_commit,
            state_checkpoint_output.state_checkpoint_hashes.clone(),
            event_hashes,
            writeset_hashes,
        );

        // Calculate root hash
        let transaction_info_hashes = transaction_infos.iter().map(CryptoHash::hash).collect_vec();
        let transaction_accumulator = Arc::new(parent_accumulator.append(&transaction_info_hashes));

        Ok(LedgerUpdateOutput::new(
            transaction_infos,
            transaction_info_hashes,
            transaction_accumulator,
            parent_accumulator,
        ))
    }

    fn calculate_events_and_writeset_hashes(
        to_commit: &[TransactionOutput],
    ) -> (Vec<HashValue>, Vec<HashValue>) {
        let _timer = OTHER_TIMERS.timer_with(&["calculate_events_and_writeset_hashes"]);

        let num_txns = to_commit.len();
        to_commit
            .par_iter()
            .with_min_len(optimal_min_len(num_txns, 64))
            .map(|txn_output| {
                let event_hashes = txn_output
                    .events()
                    .iter()
                    .map(CryptoHash::hash)
                    .collect::<Vec<_>>();

                (
                    InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash(),
                    CryptoHash::hash(txn_output.write_set()),
                )
            })
            .unzip()
    }

    fn assemble_transaction_infos(
        to_commit: &TransactionsWithOutput,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        event_hashes: Vec<HashValue>,
        writeset_hashes: Vec<HashValue>,
    ) -> Vec<TransactionInfo> {
        let _timer = OTHER_TIMERS.timer_with(&["assemble_transaction_infos"]);

        izip!(
            to_commit.iter(),
            state_checkpoint_hashes,
            event_hashes,
            writeset_hashes
        )
        .map(
            |(
                (txn, txn_out, persisted_auxiliary_info),
                state_checkpoint_hash,
                event_root_hash,
                write_set_hash,
            )| {
                // Use the auxiliary info hash directly from the persisted info
                let auxiliary_info_hash = match persisted_auxiliary_info {
                    PersistedAuxiliaryInfo::None => None,
                    PersistedAuxiliaryInfo::V1 { .. } => {
                        Some(CryptoHash::hash(persisted_auxiliary_info))
                    },
                };

                TransactionInfo::new(
                    txn.hash(),
                    write_set_hash,
                    event_root_hash,
                    state_checkpoint_hash,
                    txn_out.gas_used(),
                    txn_out.status().as_kept_status().expect("Already sorted."),
                    auxiliary_info_hash,
                )
            },
        )
        .collect()
    }
}
