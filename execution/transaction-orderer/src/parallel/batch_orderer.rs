// Copyright © Aptos Foundation

use crate::{
    batch_orderer::BatchOrderer,
    parallel::{
        min_heap::MinHeap,
        reservation_table::{
            DashMapReservationTable, MakeReservationsPhaseTrait, ParallelReservationTable,
            RemoveReservationsPhaseTrait, RequestsPhaseTrait,
        },
    },
};
use aptos_block_executor::transaction_hints::TransactionHints;
use aptos_types::{block_executor::partitioner::TxnIndex, rayontools::ExtendRef};
use itertools::Either;
use rayon::prelude::*;
use std::{
    cmp::Ordering,
    fmt::Debug,
    hash::Hash,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        RwLock,
    },
};

struct PendingTxnInfo<T> {
    transaction: T,
    pending_requests: AtomicUsize,
}

impl<T> PendingTxnInfo<T> {
    fn new(transaction: T, pending_write_table_requests: usize) -> Self {
        Self {
            transaction,
            pending_requests: pending_write_table_requests.into(),
        }
    }
}

struct SelectedTxnInfo<T> {
    transaction: T,
    index: TxnIndex,
}

impl<T> SelectedTxnInfo<T> {
    fn new(transaction: T, index: TxnIndex) -> Self {
        Self { transaction, index }
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

pub struct ParallelDynamicToposortOrderer<T: TransactionHints> {
    /// Maps transaction indices to `PendingTxnInfo<T>` for pending transactions
    /// and to `None` for selected and committed transactions.
    pending: Vec<RwLock<Option<PendingTxnInfo<T>>>>,
    active_txns_count: usize,

    // FIXME: all operations on `BTreeSet` are sequential, even with rayon.
    //        Consider using a skip list instead.
    selected: MinHeap<SelectedTxnInfo<T>>,

    write_reservations: DashMapReservationTable<T::Key, TxnIndex>,
}

impl<T> ParallelDynamicToposortOrderer<T>
where
    T: TransactionHints,
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
    T: TransactionHints,
    T::Key: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BatchOrderer for ParallelDynamicToposortOrderer<T>
where
    T: TransactionHints + Send + Sync,
    T::Key: Hash + Eq + Clone + Send + Sync + Debug,
{
    type Txn = T;

    fn add_transactions<TS>(&mut self, txns: TS)
    where
        TS: IntoIterator<Item = Self::Txn>,
    {
        // FIXME: this `collect` can be avoided, but doing so would require a separate
        //        trait for the parallel `BatchOrderer`.
        let txns: Vec<_> = txns.into_iter().collect();

        let n_new_txns = txns.len();
        let new_ids = self.pending.len()..self.pending.len() + n_new_txns;

        // reservations phase
        {
            let write_table_handle = self.write_reservations.make_reservations_phase();

            new_ids
                .clone()
                .into_par_iter()
                .zip_eq(txns.par_iter())
                .for_each(|(idx, tx)| {
                    // if idx % 1000 == 174 {
                    //     println!("================\n\n\tread_set: {:?}\n\twrite_set: {:?}\n\n================",
                    //              tx.read_set().collect::<Vec<_>>(),
                    //              tx.write_set().collect::<Vec<_>>());
                    // }
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
            new_ids
                .clone()
                .into_par_iter()
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
        let committed: Vec<SelectedTxnInfo<T>> =
            (0..count).map(|_| self.selected.pop().unwrap()).collect();

        // Update the internal data structures.
        self.active_txns_count -= count;

        let table_handle = self.write_reservations.remove_reservations_phase();

        let new_selected = committed.par_iter().flat_map(|tx_info| {
            let satisfied_write_table_requests =
                table_handle.remove_reservations(tx_info.index, tx_info.transaction.write_set());

            satisfied_write_table_requests
                .into_par_iter()
                .filter_map(|idx| {
                    let selected = {
                        let read_lock = self.pending[idx].read().unwrap();
                        let pending_info = read_lock.as_ref().unwrap();
                        let prev = pending_info.pending_requests.fetch_sub(1, SeqCst);
                        prev == 1
                    };

                    if selected {
                        let mut write_lock = self.pending[idx].write().unwrap();
                        let pending_info = write_lock.take().unwrap();
                        Some(SelectedTxnInfo::new(pending_info.transaction, idx))
                    } else {
                        None
                    }
                })
        });

        // NB: the insertion to the `BTreeSet` is done sequentially by rayon.
        self.selected.par_extend(new_selected);

        callback(
            committed
                .into_par_iter()
                .map(|info| info.transaction)
                .collect(),
        )
    }
}
