// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use dashmap::mapref::entry::Entry;
use std::hash::Hash;

use std::sync::atomic::{AtomicU32, Ordering};

/// TxnOrder represents the priorities of transactions.
/// The smaller the order, the higher the priority.
/// Two transactions in the same batch should never have the same order,
/// but there may be gaps and the order does not need to be bound by
/// the total number of transactions in a batch.
type TxnOrder = u32;

/// Allows making reservations to keys.
/// For each key, the reservation with the smallest order is kept.
pub trait ReservationTable<K> {
    /// Returns true if the reservation was made, false if the key was already reserved
    /// by a transaction with a smaller order (i.e., higher priority).
    /// Note that the `&self` reference is immutable, so this method can be called
    /// concurrently by multiple threads.
    fn make_reservation(&self, key: K, order: TxnOrder) -> bool;

    /// Returns the order of the transaction that made the reservation to this key,
    /// or None if the key is not reserved.
    fn get_reservation(&self, key: K) -> Option<TxnOrder>;
}

/// Simple `ReservationTable` implementation based on `DashMap`.
/// Acquires a write lock on a `DashMap` shard each time a reservation is made.
pub struct DashMapReservationTable<K> {
    reservations: dashmap::DashMap<K, TxnOrder>,
}

impl<K: Eq + Hash> DashMapReservationTable<K> {
    fn new() -> Self {
        Self {
            reservations: dashmap::DashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Sync + Send> ReservationTable<K> for DashMapReservationTable<K> {
    fn make_reservation(&self, key: K, order: TxnOrder) -> bool {
        // Acquires a write lock on a `DashMap` shard for the duration of this method call.
        match self.reservations.entry(key) {
            Entry::Occupied(mut entry_ref) => {
                if order < *entry_ref.get() {
                    *entry_ref.get_mut() = order;
                    true
                } else {
                    false
                }
            },
            Entry::Vacant(entry_ref) => {
                entry_ref.insert(order);
                true
            },
        }
    }

    fn get_reservation(&self, key: K) -> Option<TxnOrder> {
        // Acquires a read lock on a `DashMap` shard.
        self.reservations
            .get(&key)
            .map(|entry_ref| *entry_ref.value())
    }
}

/// Atomic version of `TxnOrder`.
type AtomicTxnOrder = AtomicU32;

/// A `DashMap`-based implementation of `ReservationTable` that optimistically
/// tries to update the reservation without a write lock using the `fetch_min` operation
/// of `AtomicTxnOrder`.
/// However, a write lock is still necessary when some key is being reserved for the first
/// time as a new entry needs to be inserted into the `DashMap`.
struct OptimisticDashMapReservationTable<K> {
    reservations: dashmap::DashMap<K, AtomicTxnOrder>,
}

impl<K: Eq + Hash> OptimisticDashMapReservationTable<K> {
    fn new() -> Self {
        Self {
            reservations: dashmap::DashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Sync + Send> ReservationTable<K> for OptimisticDashMapReservationTable<K> {
    fn make_reservation(&self, key: K, order: TxnOrder) -> bool {
        if let Some(value) = self.reservations.get(&key) {
            // Update the reservation without taking a write lock using `fetch_min`.
            // TODO: consider relaxing the ordering constraint
            let old_value = value.fetch_min(order, Ordering::SeqCst);
            order < old_value
        } else {
            // A new entry may need to be inserted into the `DashMap`.
            // Fallback to taking a write lock.
            match self.reservations.entry(key) {
                Entry::Occupied(mut entry_ref) => {
                    let value_ref = entry_ref.get_mut().get_mut();
                    if order < *value_ref {
                        *value_ref = order;
                        true
                    } else {
                        false
                    }
                },
                Entry::Vacant(entry_ref) => {
                    entry_ref.insert(AtomicTxnOrder::new(order));
                    true
                },
            }
        }
    }

    fn get_reservation(&self, key: K) -> Option<TxnOrder> {
        // TODO: consider relaxing the ordering constraint of `load`
        self.reservations
            .get(&key)
            .map(|entry_ref| entry_ref.value().load(Ordering::SeqCst))
    }
}
