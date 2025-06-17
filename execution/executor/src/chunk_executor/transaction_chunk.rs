// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{CHUNK_OTHER_TIMERS, VM_EXECUTE_CHUNK},
    workflow::do_get_execution_output::DoGetExecutionOutput,
};
use anyhow::Result;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    state::LedgerState, state_view::cached_state_view::CachedStateView,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    transaction::{AuxiliaryInfo, Transaction, TransactionOutput, Version},
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
            first_version: _,
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

        let _timer = VM_EXECUTE_CHUNK.start_timer();
        let mut auxiliary_info = Vec::new();
        // TODO(grao): Pass in persisted auxiliary info.
        auxiliary_info.resize(sig_verified_txns.len(), AuxiliaryInfo::new_empty());
        DoGetExecutionOutput::by_transaction_execution::<V>(
            &V::new(),
            sig_verified_txns.into(),
            auxiliary_info,
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
            first_version: _,
        } = self;

        DoGetExecutionOutput::by_transaction_output(
            transactions,
            transaction_outputs,
            parent_state,
            state_view,
        )
    }
}
