// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunk_executor::chunk_onchain_config,
    metrics::{CHUNK_OTHER_TIMERS, VM_EXECUTE_CHUNK},
    workflow::do_get_execution_output::DoGetExecutionOutput,
};
use anyhow::Result;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    positions::PositionParent, state::LedgerState, state_view::cached_state_view::CachedStateView,
};
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        AuxiliaryInfo, PersistedAuxiliaryInfo, Transaction, TransactionInfo, TransactionOutput,
        Version,
    },
};
use aptos_vm::VMBlockExecutor;
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::sync::Arc;

pub static SIG_VERIFY_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(8) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("chunk-sig-check-{}", index))
            .build()
            .unwrap(),
    )
});

pub struct TransactionChunk {
    pub first_version: Version,
    pub transactions: Vec<Transaction>,
    pub persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
    /// What the results of the transactions must match.
    pub transaction_infos: Vec<TransactionInfo>,
    /// Applied as is if present, otherwise the transactions are executed.
    pub transaction_outputs: Option<Vec<TransactionOutput>>,
}

impl TransactionChunk {
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_output<V: VMBlockExecutor>(
        self,
        parent_state: &LedgerState,
        parent_positions: Option<PositionParent<'_>>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        let Self {
            first_version: _,
            transactions,
            persisted_aux_info,
            transaction_infos: _,
            transaction_outputs,
        } = self;

        match transaction_outputs {
            Some(transaction_outputs) => Self::apply(
                transactions,
                transaction_outputs,
                persisted_aux_info,
                parent_state,
                parent_positions,
                state_view,
            ),
            None => Self::execute::<V>(
                transactions,
                persisted_aux_info,
                parent_state,
                parent_positions,
                state_view,
            ),
        }
    }

    fn execute<V: VMBlockExecutor>(
        transactions: Vec<Transaction>,
        persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
        parent_state: &LedgerState,
        parent_positions: Option<PositionParent<'_>>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        assert_eq!(
            transactions.len(),
            persisted_aux_info.len(),
            "transactions and persisted_aux_info must have the same length"
        );

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

        let _timer = VM_EXECUTE_CHUNK.start_timer();
        let onchain_config = chunk_onchain_config(&state_view)?;
        DoGetExecutionOutput::by_transaction_execution::<V>(
            &V::new(),
            sig_verified_txns.into(),
            persisted_aux_info
                .into_iter()
                .map(|info| AuxiliaryInfo::new(info, None))
                .collect(),
            parent_state,
            parent_positions,
            state_view,
            onchain_config,
            TransactionSliceMetadata::unknown(),
        )
    }

    fn apply(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
        parent_state: &LedgerState,
        parent_positions: Option<PositionParent<'_>>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        let onchain_config = chunk_onchain_config(&state_view)?;
        DoGetExecutionOutput::by_transaction_output(
            transactions,
            transaction_outputs,
            persisted_aux_info
                .into_iter()
                .map(|info| AuxiliaryInfo::new(info, None))
                .collect(),
            parent_state,
            parent_positions,
            state_view,
            onchain_config,
        )
    }
}
