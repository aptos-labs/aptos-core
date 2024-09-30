// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    components::chunk_output::ChunkOutput,
    metrics::{CHUNK_OTHER_TIMERS, VM_EXECUTE_CHUNK},
};
use anyhow::Result;
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    block_executor::config::BlockExecutorConfigFromOnchain,
    ledger_info::LedgerInfo,
    proof::TransactionInfoListWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use aptos_vm::VMExecutor;
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use crate::components::chunk_proof::ChunkProof;

pub static SIG_VERIFY_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(8) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("chunk-sig-check-{}", index))
            .build()
            .unwrap(),
    )
});

pub trait TransactionChunkWithProof {
    fn verify_chunk(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_version: Version,
    ) -> Result<()>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn into_output_and_proof<V: VMExecutor>(
        self,
        state_view: CachedStateView,
        verified_target_li: LedgerInfoWithSignatures,
        epoch_change_li: Option<LedgerInfoWithSignatures>,
    ) -> Result<(ChunkOutput, ChunkProof)>;
}

impl TransactionChunkWithProof for TransactionListWithProof {
    fn verify_chunk(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_version: Version,
    ) -> Result<()> {
        let _timer = CHUNK_OTHER_TIMERS.timer_with(&["verify_txn_chunk"]);

        self.proof
            .verify(ledger_info, Some(first_transaction_version))
    }

    fn len(&self) -> usize {
        self.transactions.len()
    }

    fn into_output_and_proof<V: VMExecutor>(
        self,
        state_view: CachedStateView,
        verified_target_li: LedgerInfoWithSignatures,
        epoch_change_li: Option<LedgerInfoWithSignatures>,
    ) -> Result<(ChunkOutput, ChunkProof)> {
        let TransactionListWithProof {
            transactions,
            events: _,
            first_transaction_version: _,
            proof: txn_infos_with_proof,
        } = self;

        // TODO(skedia) In the chunk executor path, we ideally don't need to verify the signature
        // as only transactions with verified signatures are committed to the storage.
        let sig_verified_txns = {
            let _timer = CHUNK_OTHER_TIMERS.timer_with(&["sig_verify"]);

            let num_txns = transactions.len();
            SIG_VERIFY_POOL.install(|| {
                transactions
                    .into_par_iter()
                    .with_min_len(optimal_min_len(num_txns, 32))
                    .map(|t| t.into())
                    .collect::<Vec<_>>()
            })
        };

        let chunk_out = {
            let _timer = VM_EXECUTE_CHUNK.start_timer();

            ChunkOutput::by_transaction_execution::<V>(
                sig_verified_txns.into(),
                state_view,
                BlockExecutorConfigFromOnchain::new_no_block_limit(),
            )?
        };
        let chunk_proof = ChunkProof {
            txn_infos_with_proof,
            verified_target_li,
            epoch_change_li,
        };

        Ok((chunk_out, chunk_proof))
    }
}

impl TransactionChunkWithProof for TransactionOutputListWithProof {
    fn verify_chunk(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_version: Version,
    ) -> Result<()> {
        self.proof
            .verify(ledger_info, Some(first_transaction_version))
    }

    fn len(&self) -> usize {
        self.transactions_and_outputs.len()
    }

    fn into_output_and_proof<V: VMExecutor>(
        self,
        state_view: CachedStateView,
        verified_target_li: LedgerInfoWithSignatures,
        epoch_change_li: Option<LedgerInfoWithSignatures>,
    ) -> Result<(ChunkOutput, ChunkProof)> {
        let TransactionOutputListWithProof {
            transactions_and_outputs,
            first_transaction_output_version: _,
            proof: txn_infos_with_proof,
        } = self;

        let chunk_out = ChunkOutput::by_transaction_output(transactions_and_outputs, state_view)?;
        let chunk_proof = ChunkProof {
            txn_infos_with_proof,
            verified_target_li,
            epoch_change_li,
        };

        Ok((chunk_out, chunk_proof))
    }
}
