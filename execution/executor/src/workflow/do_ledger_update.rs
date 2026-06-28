// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metrics::OTHER_TIMERS;
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
    transactions_with_output::TransactionsWithOutput, LedgerUpdateOutput,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    proof::accumulator::{InMemoryEventAccumulator, InMemoryTransactionAccumulator},
    transaction::{PersistedAuxiliaryInfo, TransactionInfo},
};
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

        // Assemble `TransactionInfo`s. The variant (V0 vs V1) is driven by the
        // `TRANSACTION_INFO_V1` on-chain feature, threaded via
        // `ExecutionOutput::transaction_info_v1`. The hot state root hash a V1 carries is
        // present only when `HOT_STATE_ROOT_IN_TXN_INFO` is also on (`DoStateCheckpoint`
        // produces `Some` hashes iff so); otherwise the V1 leaves it `None`.
        let (transaction_infos, transaction_info_hashes) = Self::assemble_transaction_infos(
            &execution_output.to_commit,
            execution_output.transaction_info_v1,
            &state_checkpoint_output.state_checkpoint_hashes,
            state_checkpoint_output
                .hot_state_checkpoint_hashes
                .as_deref(),
            state_checkpoint_output
                .position_state_checkpoint_hashes
                .as_deref(),
        );

        // Calculate root hash
        let transaction_accumulator = Arc::new(parent_accumulator.append(&transaction_info_hashes));

        Ok(LedgerUpdateOutput::builder()
            .transaction_infos(transaction_infos)
            .transaction_info_hashes(transaction_info_hashes)
            .transaction_accumulator(transaction_accumulator)
            .parent_accumulator(parent_accumulator)
            .build())
    }

    fn assemble_transaction_infos(
        to_commit: &TransactionsWithOutput,
        transaction_info_v1: bool,
        state_checkpoint_hashes: &[Option<HashValue>],
        hot_state_checkpoint_hashes: Option<&[Option<HashValue>]>,
        position_state_checkpoint_hashes: Option<&[Option<HashValue>]>,
    ) -> (Vec<TransactionInfo>, Vec<HashValue>) {
        let _timer = OTHER_TIMERS.timer_with(&["assemble_transaction_infos"]);

        (0..to_commit.len())
            .into_par_iter()
            .with_min_len(optimal_min_len(to_commit.len(), 64))
            .map(|i| {
                let txn = &to_commit.transactions[i];
                let txn_output = &to_commit.transaction_outputs[i];
                let persisted_auxiliary_info = &to_commit.persisted_auxiliary_infos[i];
                // Use the auxiliary info hash directly from the persisted info
                let auxiliary_info_hash = match persisted_auxiliary_info {
                    PersistedAuxiliaryInfo::None => None,
                    PersistedAuxiliaryInfo::V1 { .. } => {
                        Some(CryptoHash::hash(persisted_auxiliary_info))
                    },
                    PersistedAuxiliaryInfo::TimestampNotYetAssignedV1 { .. } => None,
                };
                let state_checkpoint_hash = state_checkpoint_hashes[i];
                let event_hashes = txn_output
                    .events()
                    .iter()
                    .map(CryptoHash::hash)
                    .collect::<Vec<_>>();
                let event_root_hash =
                    InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash();
                let write_set_hash = CryptoHash::hash(txn_output.write_set());
                let status = txn_output
                    .status()
                    .as_kept_status()
                    .expect("Already sorted.");
                let txn_info = if transaction_info_v1 {
                    TransactionInfo::builder_v1()
                        .transaction_hash(txn.committed_hash())
                        .state_change_hash(write_set_hash)
                        .event_root_hash(event_root_hash)
                        .maybe_state_checkpoint_hash(state_checkpoint_hash)
                        .maybe_hot_state_checkpoint_hash(
                            hot_state_checkpoint_hashes.and_then(|hot| hot[i]),
                        )
                        .gas_used(txn_output.gas_used())
                        .status(status)
                        .maybe_auxiliary_info_hash(auxiliary_info_hash)
                        .maybe_position_state_checkpoint_hash(
                            position_state_checkpoint_hashes.and_then(|p| p[i]),
                        )
                        .build()
                } else {
                    TransactionInfo::builder_v0()
                        .transaction_hash(txn.committed_hash())
                        .state_change_hash(write_set_hash)
                        .event_root_hash(event_root_hash)
                        .maybe_state_checkpoint_hash(state_checkpoint_hash)
                        .gas_used(txn_output.gas_used())
                        .status(status)
                        .maybe_auxiliary_info_hash(auxiliary_info_hash)
                        .build()
                };
                let txn_info_hash = txn_info.hash();
                (txn_info, txn_info_hash)
            })
            .unzip()
    }
}
