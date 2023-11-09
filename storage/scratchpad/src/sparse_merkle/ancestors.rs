// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    sparse_merkle::{dropper::SUBTREE_DROPPER, metrics::TIMER},
    SparseMerkleTree,
};
use aptos_crypto::hash::CryptoHash;
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use std::sync::Arc;

/// A container to track the ancestor of the SMTs that represent a committed state (older state is
/// guaranteed to be found in persisted storage.
/// When being queried, back pressure (a slow down) is provided in order to make sure not too many
/// SMTs are being kept in memory.
#[derive(Clone, Debug)]
pub struct SmtAncestors<V: Clone + Send + Sync + 'static> {
    youngest: Arc<Mutex<SparseMerkleTree<V>>>,
}

impl<V: CryptoHash + Clone + Send + Sync + 'static> SmtAncestors<V> {
    const MAX_PENDING_DROPS: usize = 8;

    pub fn new(ancestor: SparseMerkleTree<V>) -> Self {
        Self {
            youngest: Arc::new(Mutex::new(ancestor)),
        }
    }

    pub fn get_youngest(&self) -> SparseMerkleTree<V> {
        let _timer = TIMER.timer_with(&["get_youngest_ancestor"]);

        // The back pressure is on the getting side (which is the execution side) so that it's less
        // likely for a lot of blocks locking the same old base SMT.
        SUBTREE_DROPPER.wait_for_backlog_drop(Self::MAX_PENDING_DROPS);

        self.youngest.lock().clone()
    }

    pub fn add(&self, youngest: SparseMerkleTree<V>) {
        *self.youngest.lock() = youngest;
    }
}
