// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scheduler::{DependencyResult, Scheduler, TWaitForDependency},
    scheduler_v2::SchedulerV2,
};
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::PanicError;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Copy, Clone)]
pub(crate) enum SchedulerWrapper<'a> {
    // The AtomicBool contains a flag that determines whether to skip module reads
    // when performing validation. BlockSTMv1 uses this as an optimization to
    // avoid unnecessary work when no modules have been published. BlockSTMv2 has
    // a different validation logic, and does not require this flag. The flag is
    // stored in SchedulerWrapper only for a write (it's never read), to simplify
    // the implementation in executor.rs and avoid passing atomic booleans.
    V1(&'a Scheduler, &'a AtomicBool),
    V2(&'a SchedulerV2),
}

impl SchedulerWrapper<'_> {
    pub(crate) fn is_v2(&self) -> bool {
        matches!(self, SchedulerWrapper::V2(_))
    }

    pub(crate) fn wake_dependencies_and_decrease_validation_idx(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.wake_dependencies_and_decrease_validation_idx(txn_idx)
            },
            SchedulerWrapper::V2(_) => Ok(()),
        }
    }

    pub(crate) fn halt(&self) -> bool {
        match self {
            SchedulerWrapper::V1(scheduler, _) => scheduler.halt(),
            SchedulerWrapper::V2(scheduler) => scheduler.halt(),
        }
    }

    pub(crate) fn add_to_post_commit(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.add_to_commit_queue(txn_idx);
                Ok(())
            },
            SchedulerWrapper::V2(scheduler) => scheduler.end_commit(txn_idx),
        }
    }

    pub(crate) fn set_module_read_validation(&self) {
        match self {
            SchedulerWrapper::V1(_, skip_module_reads_validation) => {
                // Relaxed suffices as syncronization (reducing validation index) occurs after
                // setting the module read validation flag.
                skip_module_reads_validation.store(false, Ordering::Relaxed);
            },
            SchedulerWrapper::V2(_) => {},
        }
    }

    #[inline]
    pub(crate) fn interrupt_requested(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        match self {
            SchedulerWrapper::V1(scheduler, _) => scheduler.has_halted(),
            SchedulerWrapper::V2(scheduler) => scheduler.is_halted_or_aborted(txn_idx, incarnation),
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
            SchedulerWrapper::V2(_) => {
                unreachable!("SchedulerV2 does not use TWaitForDependency trait")
            },
        }
    }
}
