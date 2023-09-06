// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    common::PTransaction,
    reservation_table::{HashMapReservationTable, ReservationTable},
};
use aptos_types::block_executor::partitioner::{ITxnIndex, TxnIndex};
use std::{collections::BTreeSet, hash::Hash};

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

struct TxnInfo<T> {
    transaction: T,
    selected: bool,
    pending_write_table_requests: usize,
    pending_read_table_requests: usize,
}

/// Position of a transaction in the list of selected transactions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SelectedPosition {
    repr: ITxnIndex,
}

impl SelectedPosition {
    pub fn front(idx: TxnIndex) -> Self {
        Self {
            repr: -1 * idx as ITxnIndex,
        }
    }

    pub fn back(idx: TxnIndex) -> Self {
        Self {
            repr: idx as ITxnIndex,
        }
    }

    pub fn index(self) -> TxnIndex {
        self.repr.abs() as TxnIndex
    }
}

pub struct SequentialDynamicAriaOrderer<T: PTransaction> {
    txn_info: Vec<TxnInfo<T>>,
    active_txns_count: usize,

    selected: BTreeSet<SelectedPosition>,

    write_reservations: HashMapReservationTable<T::Key, TxnIndex>,
    read_reservations: HashMapReservationTable<T::Key, TxnIndex>,
}

impl<T: PTransaction> Default for SequentialDynamicAriaOrderer<T> {
    // NB: unfortunately, Rust cannot derive Default for generic structs
    // with type parameters that do not implement Default.
    // See: https://github.com/rust-lang/rust/issues/26925
    fn default() -> Self {
        Self {
            txn_info: Default::default(),
            active_txns_count: Default::default(),

            selected: Default::default(),

            write_reservations: Default::default(),
            read_reservations: Default::default(),
        }
    }
}

impl<T> SequentialDynamicAriaOrderer<T>
where
    T: PTransaction + Clone,
    T::Key: Hash + Eq + Clone,
{
    fn satisfy_pending_read_table_request(&mut self, idx: TxnIndex) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_read_table_requests >= 1);
        tx_info.pending_read_table_requests -= 1;

        if !tx_info.selected && tx_info.pending_read_table_requests == 0 {
            tx_info.selected = true;
            self.selected.insert(SelectedPosition::back(idx));
        }
    }

    fn satisfy_pending_write_table_request(&mut self, idx: TxnIndex) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_write_table_requests >= 1);
        tx_info.pending_write_table_requests -= 1;

        if !tx_info.selected && tx_info.pending_write_table_requests == 0 {
            tx_info.selected = true;
            self.selected.insert(SelectedPosition::back(idx));
        }
    }
}

impl<T> BatchOrderer for SequentialDynamicAriaOrderer<T>
where
    T: PTransaction + Clone,
    T::Key: Hash + Eq + Clone,
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
                .make_reservations(idx, tx.write_set());
            self.read_reservations.make_reservations(idx, tx.read_set());

            let mut selected = false;
            if self
                .write_reservations
                .are_all_satisfied(idx, tx.read_set())
            {
                // if no smaller-id dependencies, select this transaction and put it to the
                // back of the serialization order.
                selected = true;
                self.selected.insert(SelectedPosition::back(idx));
            } else if self
                .read_reservations
                .are_all_satisfied(idx, tx.write_set())
            {
                // if no smaller-id dependants, select this transaction and put it to the
                // front of the serialization order.
                selected = true;
                self.selected.insert(SelectedPosition::front(idx));
            }

            let mut pending_write_table_requests = 0;
            let mut pending_read_table_requests = 0;
            if !selected {
                pending_write_table_requests =
                    self.write_reservations.make_requests(idx, tx.read_set());
                pending_read_table_requests =
                    self.read_reservations.make_requests(idx, tx.write_set());
            }

            self.txn_info.push(TxnInfo {
                transaction: tx,
                selected,
                pending_write_table_requests,
                pending_read_table_requests,
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
            .map(|_| self.selected.pop_first().unwrap().index())
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

            let satisfied_read_table_requests = self
                .read_reservations
                .remove_reservations(committed_idx, tx.read_set());

            let satisfied_write_table_requests = self
                .write_reservations
                .remove_reservations(committed_idx, tx.write_set());

            for (idx, _) in satisfied_read_table_requests {
                self.satisfy_pending_read_table_request(idx);
            }

            for (idx, _) in satisfied_write_table_requests {
                self.satisfy_pending_write_table_request(idx);
            }
        }

        res
    }
}
