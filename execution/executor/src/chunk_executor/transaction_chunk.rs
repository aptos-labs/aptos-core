// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunk_executor::chunk_onchain_config,
    metrics::{CHUNK_DISCARDED_OUTPUTS, CHUNK_OTHER_TIMERS, VM_EXECUTE_CHUNK},
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

#[derive(Clone)]
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

    pub fn next_version(&self) -> Version {
        self.first_version + self.len() as Version
    }

    /// Appends the transactions that immediately follow.
    pub fn append(&mut self, other: Self) {
        assert_eq!(self.next_version(), other.first_version);

        self.transactions.extend(other.transactions);
        self.persisted_aux_info.extend(other.persisted_aux_info);
        self.transaction_infos.extend(other.transaction_infos);
        self.transaction_outputs =
            match (self.transaction_outputs.take(), other.transaction_outputs) {
                (Some(mut outputs), Some(other_outputs)) => {
                    outputs.extend(other_outputs);
                    Some(outputs)
                },
                // Without outputs for all the transactions, they all have to be executed.
                (outputs, other_outputs) => {
                    let num_discarded =
                        outputs.map_or(0, |o| o.len()) + other_outputs.map_or(0, |o| o.len());
                    CHUNK_DISCARDED_OUTPUTS.inc_by(num_discarded as u64);
                    None
                },
            };
    }

    /// Splits off the transactions after the last block end, i.e. the last `TransactionInfo`
    /// with a state checkpoint hash. They can't be executed until the rest of the block comes.
    pub fn split_off_partial_block(&mut self) -> Option<Self> {
        let num_txns_in_blocks = self
            .transaction_infos
            .iter()
            .rposition(TransactionInfo::has_state_checkpoint_hash)
            .map_or(0, |idx| idx + 1);
        (num_txns_in_blocks < self.len()).then(|| self.split_off(num_txns_in_blocks))
    }

    fn split_off(&mut self, at: usize) -> Self {
        Self {
            first_version: self.first_version + at as Version,
            transactions: self.transactions.split_off(at),
            persisted_aux_info: self.persisted_aux_info.split_off(at),
            transaction_infos: self.transaction_infos.split_off(at),
            transaction_outputs: self
                .transaction_outputs
                .as_mut()
                .map(|outputs| outputs.split_off(at)),
        }
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
