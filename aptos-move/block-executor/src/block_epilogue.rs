// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use aptos_mvhashmap::types::TxnIndex;
use crossbeam::utils::CachePadded;
use std::sync::MutexGuard;

/// While the writes in MVHashMap are multi-versioned, we still do (an almost
/// always uncontended) synchronization when performing shared writes. This is
/// because the block epilogue txn might be executed concurrently as some txn
/// at the same index (in case a block was cut and only partially executed).
/// The txns tries to acquire a write lock before applying shared writes to the
/// multi-versioned data-structures (if try_lock fails due to a conflict with
/// the epilogue txn, safe to early return for the calling worker as the
/// scheduler would have already halted as well). The bool indicates whether
/// the lock has been acquired by the block epilogue txn.
pub(crate) struct BlockEpilogueMutex {
    shared_writes_locks: Vec<CachePadded<Mutex<bool>>>,
}

impl BlockEpilogueMutex {
    pub(crate) fn new(num_txns: usize) -> Self {
        Self {
            shared_writes_locks: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(false)))
                .collect(),
        }
    }

    /// Called during normal speculative execution of txns.
    /// Unlock happens by dropping the MutexGuard.
    /// Returns None if the lock was previously acquired by the block epilogue txn.
    pub(crate) fn try_acquire_shared_write_lock(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<MutexGuard<'_, bool>> {
        Some(self.shared_writes_locks[txn_idx as usize].lock()).filter(|guard| !**guard)
    }

    /// Must be called by the block epilogue txn. Does not return a guard as there is
    /// a single epilogue txn and any future calls during speculative executions to
    /// [BlockEpilogueMutex::try_acquire_shared_write_lock] will fail.
    pub(crate) fn epilogue_acquire_shared_write_lock(&self, txn_idx: TxnIndex) {
        *self.shared_writes_locks[txn_idx as usize].lock() = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_epilogue_mutex_behavior() {
        let mutex = BlockEpilogueMutex::new(5);
        let txn_idx = 2;

        {
            // Before epilogue acquisition: try_acquire should succeed
            let guard1 = mutex.try_acquire_shared_write_lock(txn_idx);
            assert!(guard1.is_some_and(|guard| !*guard));
        }

        {
            let guard2 = mutex.try_acquire_shared_write_lock(txn_idx);
            assert!(guard2.is_some_and(|guard| !*guard));
        }

        // Epilogue acquires the lock
        mutex.epilogue_acquire_shared_write_lock(txn_idx);

        // After epilogue acquisition: try_acquire should fail
        let guard3 = mutex.try_acquire_shared_write_lock(txn_idx);
        assert!(guard3.is_none());

        mutex.epilogue_acquire_shared_write_lock(txn_idx);
        let guard4 = mutex.try_acquire_shared_write_lock(txn_idx);
        assert!(guard4.is_none());
    }
}
