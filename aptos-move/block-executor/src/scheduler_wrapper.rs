// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scheduler::{DependencyResult, Scheduler, TWaitForDependency},
    scheduler_v2::SchedulerV2,
};
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::PanicError;
use move_core_types::language_storage::ModuleId;
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering},
};

// Currently, OwnedSchedulerWrapper is only used in executor.rs for re-executing 
// block epilogue txn, to share the same code path as re-execution that may happen
// during sequential commit hook, e.g. when there is delayed field validation failure.
//
// The scheduler needs to be owned because block epilogue is executed after the main
// worker loop is exited, the main thread owns the scheduler and must be able to pass
// it to asynchronous dropper. as_scheduler_wrapper() method converts OwnedSchedulerWrapper
// to SchedulerWrapper, which allows using unified interfaces with BlockSTM txn execution.
//
// In case of V1, AtomicBool is initialized with true (skip module reads validation),
// and for SchedulerV2, the worker ID is set to 0. These should not have any effect
// on simple execution flow, but need to be treated with caution, such as if the inner
// state of scheduler changes, e.g. assigning more work for worker 0, or enforcing any
// other (outdated after worker loop exit) invariants.
pub(crate) enum OwnedSchedulerWrapper {
    V1(Scheduler, AtomicBool),
    V2(SchedulerV2),
}

impl OwnedSchedulerWrapper {
    pub(crate) fn from_v1(scheduler: Scheduler) -> Self {
        OwnedSchedulerWrapper::V1(scheduler, AtomicBool::new(true))
    }

    pub(crate) fn from_v2(scheduler: SchedulerV2) -> Self {
        OwnedSchedulerWrapper::V2(scheduler)
    }

    pub(crate) fn as_scheduler_wrapper(&self) -> SchedulerWrapper {
        match self {
            OwnedSchedulerWrapper::V1(scheduler, skip_module_reads_validation) => {
                SchedulerWrapper::V1(scheduler, skip_module_reads_validation)
            },
            OwnedSchedulerWrapper::V2(scheduler) => SchedulerWrapper::V2(scheduler, 0),
        }
    }
}

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
