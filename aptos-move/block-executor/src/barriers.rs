// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::types::code_invariant_error;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::delayed_fields::PanicError;
use std::sync::atomic::{AtomicU32, Ordering};

pub(crate) struct BlockBarriers {
    ready_to_start_idx: AtomicU32,
}

impl BlockBarriers {
    pub(crate) fn new() -> Self {
        Self {
            ready_to_start_idx: AtomicU32::new(0),
        }
    }

    pub(crate) fn wait_to_start(&self, txn_idx: TxnIndex) {
        while self.ready_to_start_idx.load(Ordering::Acquire) < txn_idx {}
    }

    // Transactions must be signaled to start in order. However, once a signal
    // is received subsequent signals for the same index are ignored (idempotent)
    pub(crate) fn signal_can_start(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        if txn_idx == 0 {
            return Err(code_invariant_error(
                "Txn can start signal should not be called for txn 0",
            ));
        }
        if let Err(cur_idx) = self.ready_to_start_idx.compare_exchange(
            txn_idx - 1,
            txn_idx,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            if cur_idx < txn_idx {
                return Err(code_invariant_error(format!(
                    "Txn can start signal not in order: called for {}, observed {}",
                    txn_idx, cur_idx,
                )));
            }
        }
        Ok(())
    }
}
