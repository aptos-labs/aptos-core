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
    transaction::{AuxiliaryInfo, PersistedAuxiliaryInfo, Transaction, TransactionOutput, Version},
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

    fn next_version(&self) -> Version {
        self.first_version() + self.len() as Version
    }

    fn into_output<V: VMBlockExecutor>(
        self,
        parent_state: &LedgerState,
        parent_positions: Option<PositionParent<'_>>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput>;
}

pub struct ChunkToExecute {
    pub transactions: Vec<Transaction>,
    pub persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
    pub first_version: Version,
}

impl ChunkToExecute {
    /// Splits off the transactions from index `at` on.
    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            transactions: self.transactions.split_off(at),
            persisted_aux_info: self.persisted_aux_info.split_off(at),
            first_version: self.first_version + at as Version,
        }
    }

    /// Appends the transactions right after this chunk.
    pub fn append(&mut self, mut other: Self) {
        assert_eq!(other.first_version, self.next_version());
        self.transactions.append(&mut other.transactions);
        self.persisted_aux_info
            .append(&mut other.persisted_aux_info);
    }
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
        parent_positions: Option<PositionParent<'_>>,
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
}

pub struct ChunkToApply {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
    pub first_version: Version,
}

impl ChunkToApply {
    /// Splits off the transactions from index `at` on.
    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            transactions: self.transactions.split_off(at),
            transaction_outputs: self.transaction_outputs.split_off(at),
            persisted_aux_info: self.persisted_aux_info.split_off(at),
            first_version: self.first_version + at as Version,
        }
    }

    /// Appends the transactions right after this chunk.
    pub fn append(&mut self, mut other: Self) {
        assert_eq!(other.first_version, self.next_version());
        self.transactions.append(&mut other.transactions);
        self.transaction_outputs
            .append(&mut other.transaction_outputs);
        self.persisted_aux_info
            .append(&mut other.persisted_aux_info);
    }

    /// Drops the outputs, so the transactions are executed instead.
    pub fn into_chunk_to_execute(self) -> ChunkToExecute {
        let Self {
            transactions,
            transaction_outputs: _,
            persisted_aux_info,
            first_version,
        } = self;
        ChunkToExecute {
            transactions,
            persisted_aux_info,
            first_version,
        }
    }
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
        parent_positions: Option<PositionParent<'_>>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        let Self {
            transactions,
            transaction_outputs,
            persisted_aux_info,
            first_version: _,
        } = self;

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

pub enum ChunkToExecuteOrApply {
    Execute(ChunkToExecute),
    Apply(ChunkToApply),
}

impl ChunkToExecuteOrApply {
    /// Splits off the transactions from index `at` on.
    pub fn split_off(&mut self, at: usize) -> Self {
        match self {
            Self::Execute(chunk) => Self::Execute(chunk.split_off(at)),
            Self::Apply(chunk) => Self::Apply(chunk.split_off(at)),
        }
    }

    /// Appends the transactions right after this chunk. The result is applied only if both are,
    /// since outputs are needed for all transactions to apply them.
    pub fn append(self, other: Self) -> Self {
        match (self, other) {
            (Self::Apply(mut chunk), Self::Apply(other)) => {
                chunk.append(other);
                Self::Apply(chunk)
            },
            (Self::Execute(mut chunk), other) => {
                chunk.append(other.into_chunk_to_execute());
                Self::Execute(chunk)
            },
            (Self::Apply(chunk), Self::Execute(other)) => {
                let mut chunk = chunk.into_chunk_to_execute();
                chunk.append(other);
                Self::Execute(chunk)
            },
        }
    }

    fn into_chunk_to_execute(self) -> ChunkToExecute {
        match self {
            Self::Execute(chunk) => chunk,
            Self::Apply(chunk) => chunk.into_chunk_to_execute(),
        }
    }
}

impl TransactionChunk for ChunkToExecuteOrApply {
    fn first_version(&self) -> Version {
        match self {
            Self::Execute(chunk) => chunk.first_version(),
            Self::Apply(chunk) => chunk.first_version(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Execute(chunk) => chunk.len(),
            Self::Apply(chunk) => chunk.len(),
        }
    }

    fn into_output<V: VMBlockExecutor>(
        self,
        parent_state: &LedgerState,
        parent_positions: Option<PositionParent<'_>>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        match self {
            Self::Execute(chunk) => {
                chunk.into_output::<V>(parent_state, parent_positions, state_view)
            },
            Self::Apply(chunk) => {
                chunk.into_output::<V>(parent_state, parent_positions, state_view)
            },
        }
    }
}
