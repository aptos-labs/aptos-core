// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler::{DependencyResult, Scheduler, TWaitForDependency};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::error::PanicError;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Copy, Clone)]
pub(crate) enum SchedulerWrapper<'a> {
    V1(&'a Scheduler, &'a AtomicBool),
    // TODO(BlockSTMv2): connect v2.
    #[allow(dead_code)]
    V2,
}

impl SchedulerWrapper<'_> {
    pub(crate) fn is_v2(&self) -> bool {
        matches!(self, SchedulerWrapper::V2)
    }

    pub(crate) fn wake_dependencies_and_decrease_validation_idx(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.wake_dependencies_and_decrease_validation_idx(txn_idx)
            },
            SchedulerWrapper::V2 => unimplemented!("V2 scheduler not connected in wrapper"),
        }
    }

    pub(crate) fn halt(&self) -> bool {
        match self {
            SchedulerWrapper::V1(scheduler, _) => scheduler.halt(),
            SchedulerWrapper::V2 => unimplemented!("V2 scheduler not connected in wrapper"),
        }
    }

    pub(crate) fn add_to_post_commit(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        match self {
            SchedulerWrapper::V1(scheduler, _) => {
                scheduler.add_to_commit_queue(txn_idx);
                Ok(())
            },
            SchedulerWrapper::V2 => unimplemented!("V2 scheduler not connected in wrapper"),
        }
    }

    pub(crate) fn set_module_read_validation(&self) {
        match self {
            SchedulerWrapper::V1(_, skip_module_reads_validation) => {
                // Relaxed suffices as syncronization (reducing validation index) occurs after
                // setting the module read validation flag.
                skip_module_reads_validation.store(false, Ordering::Relaxed);
            },
            SchedulerWrapper::V2 => unimplemented!("V2 scheduler not connected in wrapper"),
        }
    }

    pub(crate) fn has_halted(&self) -> bool {
        match self {
            SchedulerWrapper::V1(scheduler, _) => scheduler.has_halted(),
            SchedulerWrapper::V2 => unimplemented!("V2 scheduler not connected in wrapper"),
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
            SchedulerWrapper::V2 => {
                unreachable!("SchedulerV2 handles waiting w.o. TWaitForDependency")
            },
        }
    }
}
