// Copyright © Aptos Foundation

use aptos_channels::aptos_channel;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::Mutex;
use aptos_types::validator_txn::{Topic, ValidatorTransaction};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

pub enum TransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

impl TransactionFilter {
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

pub type VTxnPoolWrapper = Arc<Mutex<PoolState>>;

/// Create a new validator transaction pool.
pub fn new() -> VTxnPoolWrapper {
    Arc::new(Mutex::new(PoolState::default()))
}

struct PoolItem {
    seq_num: u64,
    topic: Topic,
    txn: Arc<ValidatorTransaction>,
    pull_notification_tx: Option<aptos_channel::Sender<(), Arc<ValidatorTransaction>>>,
}

/// PoolState invariants.
/// `(seq_num=i, topic=T)` exists in `txn_queue` if and only if it exists in `seq_nums_by_topic`.
#[derive(Default)]
pub struct PoolState {
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
pub struct TxnGuard {
    pool: Arc<Mutex<PoolState>>,
    seq_num: u64,
}

impl PoolState {
    /// Append a txn to the pool.
    /// Return a txn guard that allows you to later delete the txn from the pool.
    pub fn put(
        pool: Arc<Mutex<Self>>,
        topic: Topic,
        txn: Arc<ValidatorTransaction>,
        pull_notification_tx: Option<aptos_channel::Sender<(), Arc<ValidatorTransaction>>>,
    ) -> TxnGuard {
        let mut pool_guard = pool.lock();
        let seq_num = pool_guard.next_seq_num;
        pool_guard.next_seq_num += 1;

        pool_guard.txn_queue.insert(seq_num, PoolItem {
            seq_num,
            topic: topic.clone(),
            txn,
            pull_notification_tx,
        });

        if let Some(old_seq_num) = pool_guard.seq_nums_by_topic.insert(topic.clone(), seq_num) {
            pool_guard.txn_queue.remove(&old_seq_num);
        }

        TxnGuard {
            pool: pool.clone(),
            seq_num,
        }
    }

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
        while Instant::now() < deadline && max_items >= 1 && max_bytes >= 1 {
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
