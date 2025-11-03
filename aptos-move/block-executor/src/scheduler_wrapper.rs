// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor_utilities::update_transaction_on_abort,
    scheduler::{DependencyResult, Scheduler, TWaitForDependency},
    scheduler_v2::SchedulerV2,
    task::ExecutorTask,
    txn_last_input_output::TxnLastInputOutput,
};
use aptos_mvhashmap::{
    types::{Incarnation, TxnIndex},
    MVHashMap,
};
use aptos_types::{error::PanicError, transaction::BlockExecutableTransaction};
use move_core_types::language_storage::ModuleId;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Copy, Clone)]
pub(crate) enum SchedulerWrapper<'a> {
    // The AtomicBool contains a flag that determines whether to skip module reads
    // when performing validation. BlockSTMv1 uses this as an optimization to
    // avoid unnecessary work when no modules have been published. BlockSTMv2 has
    // a different validation logic, and does not require this flag. The flag is
    // stored in SchedulerWrapper only for a write (it's never read), to simplify
    // the implementation in executor.rs and avoid passing atomic booleans.
    V1(&'a Scheduler, &'a AtomicBool),
    // For V2, the usize is the worker ID which is obtained from the scheduler
    // while committing a txn.
    V2(&'a SchedulerV2, u32),
}

impl SchedulerWrapper<'_> {
    pub(crate) fn as_v2(&self) -> Option<(&SchedulerV2, u32)> {
        match self {
            SchedulerWrapper::V1(_, _) => None,
            SchedulerWrapper::V2(scheduler, worker_id) => Some((scheduler, *worker_id)),
        }
    }

    pub(crate) fn is_v2(&self) -> bool {
        matches!(self, SchedulerWrapper::V2(_, _))
    }

    pub(crate) fn wake_dependencies_and_decrease_validation_idx(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.wake_dependencies_and_decrease_validation_idx(txn_idx)
            },
            SchedulerWrapper::V2(_, _) => Ok(()),
        }
    }

    pub(crate) fn halt(&self) -> bool {
        match self {
            SchedulerWrapper::V1(scheduler, _) => scheduler.halt(),
            SchedulerWrapper::V2(scheduler, _) => scheduler.halt(),
        }
    }

    pub(crate) fn add_to_post_commit(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.add_to_commit_queue(txn_idx);
                Ok(())
            },
            SchedulerWrapper::V2(scheduler, _) => scheduler.end_commit(txn_idx),
        }
    }

    pub(crate) fn record_validation_requirements(
        &self,
        txn_idx: TxnIndex,
        module_ids: BTreeSet<ModuleId>,
    ) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(_, skip_module_reads_validation) => {
                // Relaxed suffices as syncronization (reducing validation index) occurs after
                // setting the module read validation flag.
                skip_module_reads_validation.store(false, Ordering::Relaxed);
            },
            SchedulerWrapper::V2(scheduler, worker_id) => {
                scheduler.record_validation_requirements(*worker_id, txn_idx, module_ids)?;
            },
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn interrupt_requested(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        match self {
            SchedulerWrapper::V1(scheduler, _) => scheduler.has_halted(),
            SchedulerWrapper::V2(scheduler, _) => {
                scheduler.is_halted_or_aborted(txn_idx, incarnation)
            },
        }
    }

    pub(crate) fn abort_pre_final_reexecution<T, E>(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        last_input_output: &TxnLastInputOutput<T, E::Output>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    ) -> Result<(), PanicError>
    where
        T: BlockExecutableTransaction,
        E: ExecutorTask<Txn = T>,
    {
        match self {
            SchedulerWrapper::V1(_, _) => {
                // Updating the scheduler state not required as the execute method invoked
                // in [executor::execute_txn_after_commit] does not take in the scheduler.
                update_transaction_on_abort::<T, E>(txn_idx, last_input_output, versioned_cache);
            },
            SchedulerWrapper::V2(scheduler, _) => {
                scheduler.direct_abort(txn_idx, incarnation, true)?;
            },
        }
        Ok(())
    }

    pub(crate) fn prepare_for_block_epilogue<T, E>(
        &self,
        block_epilogue_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    ) -> Result<Incarnation, PanicError>
    where
        T: BlockExecutableTransaction,
        E: ExecutorTask<Txn = T>,
    {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                let incarnation = scheduler.prepare_for_block_epilogue(block_epilogue_idx)?;
                update_transaction_on_abort::<T, E>(
                    block_epilogue_idx,
                    last_input_output,
                    versioned_cache,
                );
                Ok(incarnation)
            },
            SchedulerWrapper::V2(scheduler, _) => {
                scheduler.prepare_for_block_epilogue(block_epilogue_idx)
            },
        }
    }
}

impl TWaitForDependency for SchedulerWrapper<'_> {
    fn wait_for_dependency(
        &self,
        txn_idx: TxnIndex,
        dep_txn_idx: TxnIndex,
    ) -> Result<DependencyResult, PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.wait_for_dependency(txn_idx, dep_txn_idx)
            },
            SchedulerWrapper::V2(_, _) => {
                unreachable!("SchedulerV2 does not use TWaitForDependency trait")
            },
        }
    }
}
