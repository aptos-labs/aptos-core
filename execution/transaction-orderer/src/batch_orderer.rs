// Copyright © Aptos Foundation

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::Hash;

use aptos_types::block_executor::partitioner::TxnIndex;

use crate::common::{Direction, PTransaction};
use crate::const_option::{ConstNone, ConstOption, ConstSome};
use crate::reservation_table::{HashMapReservationTable, ReservationTable};

/// Creates batches of non-conflicting transactions.
/// Each time `commit_prefix` is called, returns a sequence of transactions
/// such that none of them read any location written by an earlier transaction in the same batch.
pub trait BatchOrderer {
    type Txn;

    /// Adds transactions to the orderer for consideration.
    fn add_transactions<TS>(&mut self, txns: TS)
    where
        TS: IntoIterator<Item = Self::Txn>;

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

/// Returns batches of non-conflicting transactions that additionally do not have dependencies
/// on transactions in recently returned batches. The exact set of transactions that the returned
/// transactions must not depend on can be regulated with the `forget_prefix` method.
pub trait BatchOrdererWithWindow: BatchOrderer {
    /// "Forgets" the `count` first not-yet-forgotten ordered transactions.
    /// When a transaction is forgotten, the orderer no longer guarantees that selected
    /// transactions do not depend on it.
    ///
    /// Each transaction goes through the following stages:
    ///     1. Active: added via `add_transactions`, but not yet returned from `commit_prefix`.
    ///     2. Recently ordered: returned from `commit_prefix`, but not yet forgotten.
    ///        Transactions returned from `commit_prefix` cannot depend on these transactions.
    ///     3. Forgotten: no longer considered by the orderer. Transactions returned from
    ///        `commit_prefix` are allowed to depend on these transactions.
    ///
    /// Note that `self.count_selected()` will not increase unless `forget_prefix` is called.
    ///
    /// `count` must not be greater than `self.get_window_size()`.
    fn forget_prefix(&mut self, count: usize);

    /// Returns the number of not-yet-forgotten ordered transactions.
    fn get_window_size(&self) -> usize;
}

struct TxnInfo<T> {
    transaction: T,
    selected: bool,
    pending_write_table_requests: usize,
    pending_read_table_requests: usize,
    pending_recent_write_dependencies: usize,
}

#[derive(Default)]
struct RecentWriteInfo {
    count: usize,
    dependencies: HashSet<TxnIndex>,
}

pub struct WindowManager<K> {
    recently_committed_txns: VecDeque<TxnIndex>,
    recent_writes: HashMap<K, RecentWriteInfo>,
}

impl<K> Default for WindowManager<K> {
    fn default() -> Self {
        Self {
            recently_committed_txns: VecDeque::new(),
            recent_writes: HashMap::new(),
        }
    }
}

pub struct SequentialDynamicAriaOrderer<T: PTransaction, WM> {
    txn_info: Vec<TxnInfo<T>>,
    active_txns_count: usize,

    selected: BTreeSet<(Direction, TxnIndex)>,

    write_reservations: HashMapReservationTable<T::Key, TxnIndex>,
    read_reservations: HashMapReservationTable<T::Key, TxnIndex>,

    window: WM, // ConstOption<WindowManager<T::Key>>
}

impl<T: PTransaction> SequentialDynamicAriaOrderer<T, ConstSome<WindowManager<T::Key>>> {
    pub fn with_window() -> Self {
        Self {
            txn_info: Vec::new(),
            active_txns_count: 0,
            selected: BTreeSet::new(),
            write_reservations: HashMapReservationTable::default(),
            read_reservations: HashMapReservationTable::default(),
            window: ConstSome(WindowManager::default()),
        }
    }
}

impl<T: PTransaction> SequentialDynamicAriaOrderer<T, ConstNone> {
    pub fn without_window() -> Self {
        Self {
            txn_info: Vec::new(),
            active_txns_count: 0,
            selected: BTreeSet::new(),
            write_reservations: HashMapReservationTable::default(),
            read_reservations: HashMapReservationTable::default(),
            window: ConstNone(),
        }
    }
}

impl<T, WM> SequentialDynamicAriaOrderer<T, WM>
where
    T: PTransaction + Clone,
    T::Key: Ord + Hash + Eq + Clone,
    WM: ConstOption<WindowManager<T::Key>>,
{
    fn satisfy_pending_read_table_request(&mut self, idx: TxnIndex) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_read_table_requests >= 1);
        tx_info.pending_read_table_requests -= 1;
        if !tx_info.selected
            && tx_info.pending_read_table_requests == 0
            && tx_info.pending_recent_write_dependencies == 0
        {
            tx_info.selected = true;
            self.selected.insert((Direction::Back, idx));
        }
    }

    fn satisfy_pending_write_table_request(&mut self, idx: TxnIndex, key: &T::Key) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_write_table_requests >= 1);
        tx_info.pending_write_table_requests -= 1;

        match self.window.as_option_mut() {
            Some(window) => {
                // If window is enabled, register the dependency on the recent write.
                // This transaction cannot be committed until this dependency is satisfied.
                if window
                    .recent_writes
                    .get_mut(key)
                    .unwrap()
                    .dependencies
                    .insert(idx)
                {
                    tx_info.pending_recent_write_dependencies += 1;
                }
            },
            None => {
                // If window is disabled and a transaction has no more pending requests
                // in the write table, it can be selected.
                if !tx_info.selected && tx_info.pending_write_table_requests == 0 {
                    tx_info.selected = true;
                    self.selected.insert((Direction::Back, idx));
                }
            },
        }
    }

    fn resolve_recent_write_dependency(&mut self, idx: TxnIndex) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_recent_write_dependencies >= 1);
        tx_info.pending_recent_write_dependencies -= 1;
        if tx_info.pending_recent_write_dependencies == 0 {
            assert!(!tx_info.selected);

            if tx_info.pending_read_table_requests == 0 {
                tx_info.selected = true;
                self.selected.insert((Direction::Back, idx));
            } else if tx_info.pending_write_table_requests == 0 {
                tx_info.selected = true;
                self.selected.insert((Direction::Front, idx));
            }
        }
    }
}

impl<T, WM> BatchOrderer for SequentialDynamicAriaOrderer<T, WM>
where
    T: PTransaction + Clone,
    T::Key: Ord + Hash + Eq + Clone,
    WM: ConstOption<WindowManager<T::Key>>,
{
    type Txn = T;

    fn add_transactions<TS>(&mut self, txns: TS)
    where
        TS: IntoIterator<Item = Self::Txn>,
    {
        for tx in txns {
            let idx = self.txn_info.len();
            self.active_txns_count += 1;

            self.write_reservations
                .make_reservations(idx, &tx.write_set());
            self.read_reservations
                .make_reservations(idx, &tx.read_set());

            let mut pending_recent_write_dependencies = 0;
            if let Some(window) = self.window.as_option_mut() {
                if !window.recent_writes.is_empty() {
                    for k in tx.write_set() {
                        if let Some(write_info) = window.recent_writes.get_mut(&k) {
                            pending_recent_write_dependencies += 1;
                            write_info.dependencies.insert(idx);
                        }
                    }
                }
            }

            let mut selected = false;
            if pending_recent_write_dependencies == 0 {
                if self
                    .write_reservations
                    .are_all_satisfied(idx, &tx.read_set())
                {
                    // if no smaller-id dependencies, select this transaction and put it to the
                    // back of the serialization order.
                    selected = true;
                    self.selected.insert((Direction::Back, idx));
                } else if self
                    .read_reservations
                    .are_all_satisfied(idx, &tx.write_set())
                {
                    // if no smaller-id dependants, select this transaction and put it to the
                    // front of the serialization order.
                    selected = true;
                    self.selected.insert((Direction::Front, idx));
                }
            }

            let mut pending_write_table_requests = 0;
            let mut pending_read_table_requests = 0;
            if !selected {
                pending_write_table_requests =
                    self.write_reservations.make_requests(idx, &tx.read_set());
                pending_read_table_requests =
                    self.read_reservations.make_requests(idx, &tx.write_set());
            }

            self.txn_info.push(TxnInfo {
                transaction: tx,
                selected,
                pending_write_table_requests,
                pending_read_table_requests,
                pending_recent_write_dependencies,
            });
        }
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

        let committed_indices: Vec<_> = (0..count)
            .map(|_| self.selected.pop_first().unwrap().1)
            .collect();

        let committed_txns: Vec<_> = committed_indices
            .iter()
            .map(|&idx| self.txn_info[idx].transaction.clone())
            .collect();

        // Return the committed transactions early via the callback, to minimize latency.
        // Note that the callback cannot access the Orderer as we are still holding a mutable
        // reference to it. Hence, it will not be able to observe the orderer in an inconsistent
        // state.
        let res = callback(committed_txns);

        // Update the internal data structures.
        self.active_txns_count -= count;

        for &committed_idx in committed_indices.iter() {
            let tx = &self.txn_info[committed_idx].transaction;
            let write_set = tx.write_set();

            if let Some(window) = self.window.as_option_mut() {
                window.recently_committed_txns.push_back(committed_idx);
                for key in &write_set {
                    window.recent_writes.entry(key.clone()).or_default().count += 1;
                }
            }

            let satisfied_write_table_requests = self
                .write_reservations
                .remove_reservations(committed_idx, &write_set);

            for (idx, key) in satisfied_write_table_requests {
                self.satisfy_pending_write_table_request(idx, &key);
            }
        }

        // The read table requests have to be processed after all the write table requests,
        // are processed to make sure that `pending_recent_write_dependencies` is updated.
        for &committed_idx in committed_indices.iter() {
            let tx = &self.txn_info[committed_idx].transaction;

            let satisfied_read_table_requests = self
                .read_reservations
                .remove_reservations(committed_idx, &tx.read_set());

            for (idx, _) in satisfied_read_table_requests {
                self.satisfy_pending_read_table_request(idx);
            }
        }

        res
    }
}

impl<T> BatchOrdererWithWindow for SequentialDynamicAriaOrderer<T, ConstSome<WindowManager<T::Key>>>
where
    T: PTransaction + Clone,
    T::Key: Ord + Hash + Eq + Clone,
{
    fn forget_prefix(&mut self, count: usize) {
        assert!(count <= self.get_window_size());
        let forgotten_indices = self
            .window
            .recently_committed_txns
            .drain(0..count)
            .collect::<Vec<_>>();

        for forgotten_idx in forgotten_indices {
            let tx = &self.txn_info[forgotten_idx].transaction;
            for key in tx.write_set() {
                let write_info = self.window.0.recent_writes.get_mut(&key).unwrap();
                write_info.count -= 1;
                if write_info.count == 0 {
                    let write_info = self.window.0.recent_writes.remove(&key).unwrap();
                    for &idx in write_info.dependencies.iter() {
                        self.resolve_recent_write_dependency(idx);
                    }
                }
            }
        }
    }

    fn get_window_size(&self) -> usize {
        self.window.0.recently_committed_txns.len()
    }
}
