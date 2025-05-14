// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// TODO(BlockSTMv2): enable dead code lint.
#![allow(dead_code)]

use aptos_infallible::Mutex;
use aptos_mvhashmap::types::TxnIndex;
use crossbeam::utils::CachePadded;
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU32, Ordering},
};

/// A structure that manages the execution queue for the BlockSTMv2 scheduler, and also
/// exposes proxy interfaces to its implementation details (e.g. scheduler status).
pub(crate) struct ExecutionQueueManager {
    /// The first executed_once_max_idx many transctions have all finished their first
    /// incarnation, i.e. have been executed at least once.
    executed_once_max_idx: CachePadded<AtomicU32>,
    /// Queue for scheduling transactions for execution.
    /// TODO(BlockSTMv2): Alternative implementations for performance (e.g. packed ints,
    /// intervals w. locks, CachePadded<ConcurrentQueue<TxnIndex>>).
    execution_queue: Mutex<BTreeSet<TxnIndex>>,
}

impl ExecutionQueueManager {
    // Note: is_first_reexecution must be determined and the method must be performed
    // while holding the idx-th status lock.
    pub(crate) fn add_to_schedule(&self, is_first_reexecution: bool, txn_idx: TxnIndex) {
        // TODO(BlockSTMv2): Explain the logic for is_first_reexecution when SchedulerV2
        // logic is added to the file.
        if !is_first_reexecution || self.executed_once_max_idx.load(Ordering::Relaxed) >= txn_idx {
            self.execution_queue.lock().insert(txn_idx);
        }
    }

    pub(crate) fn remove_from_schedule(&self, txn_idx: TxnIndex) {
        self.execution_queue.lock().remove(&txn_idx);
    }
}

// Testing interfaces for ExecutionQueueManager.
impl ExecutionQueueManager {
    #[cfg(test)]
    pub(crate) fn new_for_test(executed_once_max_idx: u32) -> Self {
        Self {
            executed_once_max_idx: CachePadded::new(AtomicU32::new(executed_once_max_idx)),
            execution_queue: Mutex::new(BTreeSet::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn assert_execution_queue(&self, expected_indices: &Vec<TxnIndex>) {
        let queue = self.execution_queue.lock();
        assert_eq!(queue.len(), expected_indices.len());
        for scheduled_idx in expected_indices {
            assert!(queue.contains(scheduled_idx));
        }
    }
}
