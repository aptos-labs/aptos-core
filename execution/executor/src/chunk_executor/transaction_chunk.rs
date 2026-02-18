// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics::{CHUNK_OTHER_TIMERS, VM_EXECUTE_CHUNK},
    workflow::do_get_execution_output::DoGetExecutionOutput,
};
use anyhow::Result;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_experimental_runtimes::thread_manager::{optimal_min_len, THREAD_MANAGER};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    state::LedgerState, state_view::cached_state_view::CachedStateView,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    transaction::{AuxiliaryInfo, PersistedAuxiliaryInfo, Transaction, TransactionOutput, Version},
};
use aptos_vm::VMBlockExecutor;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

pub trait TransactionChunk {
    fn first_version(&self) -> Version;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn into_output<V: VMBlockExecutor>(
        self,
        parent_state: &LedgerState,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput>;
}

pub struct ChunkToExecute {
    pub transactions: Vec<Transaction>,
    pub persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
    pub first_version: Version,
}

impl TransactionChunk for ChunkToExecute {
    fn first_version(&self) -> Version {
        self.first_version
    }

    fn len(&self) -> usize {
        self.transactions.len()
    }

    fn into_output<V: VMBlockExecutor>(
        self,
        parent_state: &LedgerState,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        let ChunkToExecute {
            transactions,
            persisted_aux_info,
            first_version: _,
        } = self;

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
            THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                transactions
                    .into_par_iter()
                    .with_min_len(optimal_min_len(num_txns, 32))
                    .map(|t| t.into())
                    .collect::<Vec<_>>()
            })
        };

        let _timer = VM_EXECUTE_CHUNK.start_timer();
        DoGetExecutionOutput::by_transaction_execution::<V>(
            &V::new(),
            sig_verified_txns.into(),
            persisted_aux_info
                .into_iter()
                .map(|info| AuxiliaryInfo::new(info, None))
                .collect(),
            parent_state,
            state_view,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
            TransactionSliceMetadata::unknown(),
        )
    }
}

pub struct ChunkToApply {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
    pub first_version: Version,
}

impl TransactionChunk for ChunkToApply {
    fn first_version(&self) -> Version {
        self.first_version
    }

    fn len(&self) -> usize {
        self.transactions.len()
    }

    fn into_output<V: VMBlockExecutor>(
        self,
        parent_state: &LedgerState,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        let Self {
            transactions,
            transaction_outputs,
            persisted_aux_info,
            first_version: _,
        } = self;

        DoGetExecutionOutput::by_transaction_output(
            transactions,
            transaction_outputs,
            persisted_aux_info
                .into_iter()
                .map(|info| AuxiliaryInfo::new(info, None))
                .collect(),
            parent_state,
            state_view,
        )
    }
}
