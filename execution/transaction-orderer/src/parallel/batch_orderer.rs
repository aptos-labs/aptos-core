// Copyright © Aptos Foundation

use std::{
    hash::Hash,
};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::RwLock;
use itertools::{Either};

use rayon::{prelude::*};

use aptos_types::block_executor::partitioner::TxnIndex;
use aptos_types::rayontools::ExtendRef;

use crate::{
    common::{PTransaction},
    parallel::reservation_table::{
        DashMapReservationTable, ParallelReservationTable, RequestsPhaseTrait,
        ReservationsPhaseTrait,
    },
};

/// Creates batches of non-conflicting transactions.
/// Each time `commit_prefix` is called, returns a sequence of transactions
/// such that none of them read any location written by an earlier transaction in the same batch.
pub trait BatchOrderer {
    type Txn;

    /// Adds transactions to the orderer for consideration.
    fn add_transactions(&mut self, txns: Vec<Self::Txn>);

    /// Returns the number of transactions that have been added to the orderer but not yet ordered.
    fn count_active_transactions(&self) -> usize;

    /// Returns `true` if all added transactions were ordered.
    /// Equivalent to `self.count_active_transactions() == 0`.
    fn is_empty(&self) -> bool {
        self.count_active_transactions() == 0
    }

    /// Returns the maximum size of a batch of non-conflicting transactions that the orderer is
    /// ready to produce.
    fn count_selected(&self) -> usize;

    /// Calls `callback` on a sequence of non-conflicting transactions and removes them from
    /// the orderer. This may lead to an increase of `count_selected()` as transactions that
    /// conflict with the returned transactions can now be selected.
    fn commit_prefix_callback<F, R>(&mut self, count: usize, callback: F) -> R
    where
        F: FnOnce(Vec<Self::Txn>) -> R;

    /// Returns a sequence of non-conflicting transactions and removes them from the orderer.
    fn commit_prefix(&mut self, count: usize) -> Vec<Self::Txn> {
        let mut committed = Vec::new();
        self.commit_prefix_callback(count, |txns| committed = txns);
        committed
    }
}

struct PendingTxnInfo<T> {
    transaction: T,
    pending_write_table_requests: AtomicUsize,
}


impl<T> PendingTxnInfo<T> {
    fn new(transaction: T, pending_write_table_requests: usize) -> Self {
        Self {
            transaction,
            pending_write_table_requests: pending_write_table_requests.into(),
        }
    }
}

struct SelectedTxnInfo<T> {
    transaction: T,
    index: TxnIndex,
}

impl<T> SelectedTxnInfo<T> {
    fn new(transaction: T, index: TxnIndex) -> Self {
        Self {
            transaction,
            index,
        }
    }
}

impl<T> PartialEq for SelectedTxnInfo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for SelectedTxnInfo<T> {}

impl<T> PartialOrd for SelectedTxnInfo<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.index.cmp(&other.index))
    }
}

impl<T> Ord for SelectedTxnInfo<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

pub struct ParallelDynamicToposortOrderer<T: PTransaction> {
    /// Maps transaction indices to `PendingTxnInfo<T>` for pending transactions
    /// and to `None` for selected and committed transactions.
    pending: Vec<RwLock<Option<PendingTxnInfo<T>>>>,
    active_txns_count: usize,

    // FIXME: all operations on `BTreeSet` are sequential, even with rayon.
    //        Consider using a skip list instead.
    selected: BTreeSet<SelectedTxnInfo<T>>,

    write_reservations: DashMapReservationTable<T::Key, TxnIndex>,
}

impl<T> ParallelDynamicToposortOrderer<T>
where
    T: PTransaction,
    T::Key: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            pending: Default::default(),
            active_txns_count: 0,
            selected: Default::default(),
            write_reservations: Default::default(),
        }
    }
}

impl<T> Default for ParallelDynamicToposortOrderer<T>
where
    T: PTransaction,
    T::Key: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BatchOrderer for ParallelDynamicToposortOrderer<T>
where
    T: PTransaction + Send + Sync,
    T::Key: Hash + Eq + Clone + Send + Sync,
{
    type Txn = T;

    fn add_transactions(&mut self, txns: Vec<Self::Txn>) {
        let n_new_txns = txns.len();
        let new_ids = self.pending.len()..self.pending.len() + n_new_txns;

        // reservations phase
        {
            let write_table_handle = self.write_reservations.reservations_phase();

            new_ids.clone()
                .into_par_iter()
                .zip_eq(txns.par_iter())
                .for_each(|(idx, tx)| {
                    write_table_handle.make_reservations(idx, tx.write_set());
                });
        }

        // requests phase
        let write_table_handle = self.write_reservations.requests_phase();

        // `ExtendRef` is used to extend multiple collections from one rayon iterator.
        let pending = ExtendRef::new(&mut self.pending);
        let selected = ExtendRef::new(&mut self.selected);

        // NB: insertions to the `BTreeSet` are done sequentially by rayon.
        (pending, (selected, ())).par_extend(
            new_ids.clone().into_par_iter()
                .zip_eq(txns.into_par_iter())
                .map(|(idx, tx)| {
                    let pending_requests = write_table_handle.make_requests(idx, tx.read_set());
                    let selected = pending_requests == 0;

                    if selected {
                        (
                            // Insert `None` to `self.pending`.
                            RwLock::new(None),
                            // Insert `SelectedTxnInfo` to `self.selected`.
                            Either::Left(SelectedTxnInfo::new(tx, idx)),
                        )
                    } else {
                        (
                            // Insert `Some(PendingTxnInfo)` to `self.pending`.
                            RwLock::new(Some(PendingTxnInfo::new(tx, pending_requests))),
                            // Do not insert anything to `self.selected`.
                            Either::Right(()),
                        )
                    }
                }),
        );

        self.active_txns_count += n_new_txns;
    }

    fn count_active_transactions(&self) -> usize {
        self.active_txns_count
    }

    fn count_selected(&self) -> usize {
        self.selected.len()
    }

    fn commit_prefix_callback<F, R>(&mut self, count: usize, callback: F) -> R
    where
        F: FnOnce(Vec<Self::Txn>) -> R,
    {
        assert!(count <= self.count_selected());

        // NB: this step is done sequentially.
        // Ideally, to parallelize this step, we would need something like a concurrent
        // priority queue with support for fast `split_off` by index.
        // This is possible to implement, but does not seem to be readily available in any
        // widely used Rust crate.
        let committed: Vec<SelectedTxnInfo<T>> = (0..count)
            .map(|_| self.selected.pop_first().unwrap())
            .collect();

        // Update the internal data structures.
        self.active_txns_count -= count;

        let table_handle = self.write_reservations.reservations_phase();

        let new_selected = committed.par_iter()
            .flat_map(|tx_info| {
                let satisfied_write_table_requests =
                    table_handle.remove_reservations(tx_info.index, tx_info.transaction.write_set());

                satisfied_write_table_requests
                    .into_par_iter()
                    .filter_map(|idx| {
                        let selected = {
                            let read_lock = self.pending[idx].read().unwrap();
                            let pending_info = read_lock.as_ref().unwrap();
                            let prev = pending_info.pending_write_table_requests.fetch_sub(1, SeqCst);
                            prev == 1
                        };

                        if selected {
                            let mut write_lock = self.pending[idx].write().unwrap();
                            let pending_info = write_lock.take().unwrap();
                            Some(SelectedTxnInfo::new(
                                pending_info.transaction,
                                idx,
                            ))
                        } else {
                            None
                        }
                    })
            });

        // NB: this step is done sequentially.
        self.selected.par_extend(new_selected);

        callback(committed.into_par_iter().map(|info| info.transaction).collect())
    }
}
