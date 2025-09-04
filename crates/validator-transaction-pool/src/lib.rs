// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_channels::velor_channel;
use velor_crypto::{hash::CryptoHash, HashValue};
use velor_infallible::Mutex;
use velor_types::validator_txn::{Topic, ValidatorTransaction};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::{Debug, Formatter},
    sync::Arc,
    time::Instant,
};

pub enum TransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

impl TransactionFilter {
    pub fn no_op() -> Self {
        Self::PendingTxnHashSet(HashSet::new())
    }
}

impl TransactionFilter {
    pub fn empty() -> Self {
        Self::PendingTxnHashSet(HashSet::new())
    }

    pub fn should_exclude(&self, txn: &ValidatorTransaction) -> bool {
        match self {
            TransactionFilter::PendingTxnHashSet(set) => set.contains(&txn.hash()),
        }
    }
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self::PendingTxnHashSet(HashSet::new())
    }
}

#[derive(Clone)]
pub struct VTxnPoolState {
    inner: Arc<Mutex<PoolStateInner>>,
}

impl Default for VTxnPoolState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PoolStateInner::default())),
        }
    }
}
impl VTxnPoolState {
    /// Append a txn to the pool.
    /// Return a txn guard that allows you to later delete the txn from the pool.
    pub fn put(
        &self,
        topic: Topic,
        txn: Arc<ValidatorTransaction>,
        pull_notification_tx: Option<velor_channel::Sender<(), Arc<ValidatorTransaction>>>,
    ) -> TxnGuard {
        let mut pool = self.inner.lock();
        let seq_num = pool.next_seq_num;
        pool.next_seq_num += 1;

        pool.txn_queue.insert(seq_num, PoolItem {
            topic: topic.clone(),
            txn,
            pull_notification_tx,
        });

        if let Some(old_seq_num) = pool.seq_nums_by_topic.insert(topic.clone(), seq_num) {
            pool.txn_queue.remove(&old_seq_num);
        }

        TxnGuard {
            pool: self.inner.clone(),
            seq_num,
        }
    }

    pub fn pull(
        &self,
        deadline: Instant,
        max_items: u64,
        max_bytes: u64,
        filter: TransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        self.inner
            .lock()
            .pull(deadline, max_items, max_bytes, filter)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_txn_guard(&self) -> TxnGuard {
        TxnGuard {
            pool: self.inner.clone(),
            seq_num: u64::MAX,
        }
    }
}

struct PoolItem {
    topic: Topic,
    txn: Arc<ValidatorTransaction>,
    pull_notification_tx: Option<velor_channel::Sender<(), Arc<ValidatorTransaction>>>,
}

/// PoolState invariants.
/// `(seq_num=i, topic=T)` exists in `txn_queue` if and only if it exists in `seq_nums_by_topic`.
#[derive(Default)]
pub struct PoolStateInner {
    /// Incremented every time a txn is pushed in. The txn gets the old value as its sequence number.
    next_seq_num: u64,

    /// Track Topic -> seq_num mapping.
    /// We allow only 1 txn per topic and this index helps find the old txn when adding a new one for the same topic.
    seq_nums_by_topic: HashMap<Topic, u64>,

    /// Txns ordered by their sequence numbers (i.e. time they entered the pool).
    txn_queue: BTreeMap<u64, PoolItem>,
}

/// Returned for `txn` when you call `PoolState::put(txn, ...)`.
/// If this is dropped, `txn` will be deleted from the pool (if it has not been).
///
/// This allows the pool to be emptied on epoch boundaries.
#[derive(Clone)]
pub struct TxnGuard {
    pool: Arc<Mutex<PoolStateInner>>,
    seq_num: u64,
}

impl Debug for TxnGuard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TxnGuard")
            .field("seq_num", &self.seq_num)
            .finish()
    }
}

impl PoolStateInner {
    fn try_delete(&mut self, seq_num: u64) {
        if let Some(item) = self.txn_queue.remove(&seq_num) {
            let seq_num_another = self.seq_nums_by_topic.remove(&item.topic);
            assert_eq!(Some(seq_num), seq_num_another);
        }
    }

    pub fn pull(
        &mut self,
        deadline: Instant,
        mut max_items: u64,
        mut max_bytes: u64,
        filter: TransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        let mut ret = vec![];
        let mut seq_num_lower_bound = 0;

        // Check deadline at the end of every iteration to ensure validator txns get a chance no matter what current proposal delay is.
        while max_items >= 1 && max_bytes >= 1 {
            // Find the seq_num of the first txn that satisfies the quota.
            if let Some(seq_num) = self
                .txn_queue
                .range(seq_num_lower_bound..)
                .filter(|(_, item)| {
                    item.txn.size_in_bytes() as u64 <= max_bytes
                        && !filter.should_exclude(&item.txn)
                })
                .map(|(seq_num, _)| *seq_num)
                .next()
            {
                // Update the quota usage.
                // Send the pull notification if requested.
                let PoolItem {
                    txn,
                    pull_notification_tx,
                    ..
                } = self.txn_queue.get(&seq_num).unwrap();
                if let Some(tx) = pull_notification_tx {
                    let _ = tx.push((), txn.clone());
                }
                max_items -= 1;
                max_bytes -= txn.size_in_bytes() as u64;
                seq_num_lower_bound = seq_num + 1;
                ret.push(txn.as_ref().clone());

                if Instant::now() >= deadline {
                    break;
                }
            } else {
                break;
            }
        }

        ret
    }
}

impl Drop for TxnGuard {
    fn drop(&mut self) {
        self.pool.lock().try_delete(self.seq_num);
    }
}

#[cfg(test)]
mod tests;
