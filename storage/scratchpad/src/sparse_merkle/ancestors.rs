// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{sparse_merkle::metrics::TIMER, SparseMerkleTree};
use aptos_crypto::hash::CryptoHash;
use aptos_drop_helper::async_drop_queue::AsyncDropQueue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use std::{
    collections::VecDeque,
    sync::{Arc, MutexGuard},
    time::Duration,
};

type Result<V> = std::result::Result<V, Error>;

/// Keep the oldest SMTs in a central place so that:
/// 1. elsewhere, dropping the SMT will be fast since this indirectly holds a ref to every tree in
///    the entire forest.
/// 2. this must be invoked somewhere in the critical path so that it can provide some
///    back pressure to slow things down to prevent from leaking memory
#[derive(Clone, Debug)]
pub struct SmtAncestors<V: Clone + Send + Sync + 'static> {
    /// Keep a queue of ancestors, in hope that the when the oldest is being evicted, it's the
    /// the last ref of it, which means the drop latency doesn't impact other code paths.
    ancestors: Arc<Mutex<VecDeque<SparseMerkleTree<V>>>>,
    /// Drop the oldest ancestor asynchronously in good cases, with limited backlog, providing
    /// back pressure when the drops are slow in order to avoid memory leak.
    drop_queue: Arc<AsyncDropQueue>,
}

impl<V: CryptoHash + Clone + Send + Sync + 'static> SmtAncestors<V> {
    const MAX_PENDING_DROPS: usize = 4;
    const NUM_ANCESTORS: usize = 8;

    pub fn new(ancestor: SparseMerkleTree<V>) -> Self {
        Self {
            ancestors: Arc::new(Mutex::new(VecDeque::from(vec![ancestor]))),
            drop_queue: Arc::new(AsyncDropQueue::new(
                "smt_ancestors",
                Self::MAX_PENDING_DROPS,
            )),
        }
    }

    fn ancestors(&self) -> MutexGuard<VecDeque<SparseMerkleTree<V>>> {
        self.ancestors.lock()
    }

    pub fn get_youngest(&self) -> Result<SparseMerkleTree<V>> {
        self.ancestors()
            .back()
            .map(SparseMerkleTree::clone)
            .ok_or(Error::NotFound)
    }

    pub fn add(&self, youngest: SparseMerkleTree<V>) {
        let _timer = TIMER.timer_with(&["add_smt_ancestor"]);

        let mut ancestors = self.ancestors();
        ancestors.push_back(youngest);

        if ancestors.len() > Self::NUM_ANCESTORS {
            let oldest = ancestors.pop_front().unwrap();
            oldest.log_generation("evict_ancestor");
            if !oldest.is_the_only_ref() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    error!(
                        "Oldest SMT being tracked by SmtAncestors is still referenced elsewhere. Potential memory leak.",
                    )
                );
            } else {
                self.drop_queue.enqueue_drop(oldest);
            }
        }
    }

    pub fn replace_with(&self, other: SmtAncestors<V>) {
        let Self {
            ancestors,
            drop_queue: _drop_queue,
        } = other;

        *self.ancestors.lock() = Arc::into_inner(ancestors)
            .expect("Not the only ref.")
            .into_inner();
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    #[error("Ancestor SMT not found.")]
    NotFound,
}
