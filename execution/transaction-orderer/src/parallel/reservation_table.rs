// Copyright © Aptos Foundation

use std::{
    collections::{HashSet, VecDeque},
    hash::Hash,
};
use std::collections::BTreeSet;

use dashmap::DashMap;
use crate::parallel::min_heap::{MinHeap, MinHeapWithRemove};

pub trait MakeReservationsPhaseTrait {
    type Key;
    type TxnId: Copy;

    /// Adds a reservation to the table.
    fn make_reservation(&self, idx: Self::TxnId, key: &Self::Key);

    /// Adds reservations to the table.
    fn make_reservations<'a, KS>(&self, idx: Self::TxnId, keys: KS)
    where
        Self::Key: 'a,
        KS: IntoIterator<Item = &'a Self::Key>,
    {
        for key in keys.into_iter() {
            self.make_reservation(idx, key);
        }
    }
}

pub trait RemoveReservationsPhaseTrait {
    type Key;
    type TxnId: Copy;

    // TODO: consider returning an iterator instead of a vector
    /// Removes a reservation from the table and returns the requests that
    /// are now satisfied and removes them from the table.
    fn remove_reservation(
        &self,
        idx: Self::TxnId,
        keys: &Self::Key,
    ) -> Vec<Self::TxnId>;

    // TODO: consider returning an iterator instead of a vector
    /// Removes reservations from the table and returns the requests that
    /// are now satisfied and removes them from the table.
    fn remove_reservations<'a, KS>(
        &self,
        idx: Self::TxnId,
        keys: KS,
    ) -> Vec<Self::TxnId>
    where
        Self::Key: 'a,
        KS: IntoIterator<Item = &'a Self::Key>,
    {
        let mut res = Vec::new();
        for key in keys.into_iter() {
            res.append(&mut self.remove_reservation(idx, key));
        }
        res
    }
}

pub trait RequestsPhaseTrait {
    type Key;
    type TxnId: Copy;

    /// Tries to add a pending request to the table.
    /// Returns `true` if a pending request is added and false if it is already satisfied.
    fn make_request(&self, idx: Self::TxnId, key: &Self::Key) -> bool;

    /// Tries to add pending requests to the table.
    /// Returns the number of added pending requests (i.e., the number of requests in the input
    /// that are not already satisfied).
    fn make_requests<'a, KS>(&self, idx: Self::TxnId, keys: KS) -> usize
    where
        Self::Key: 'a,
        KS: IntoIterator<Item = &'a Self::Key>,
    {
        keys.into_iter()
            .filter(|&key| self.make_request(idx, key))
            .count()
    }

    /// Checks whether the request is satisfied without adding it as a pending request
    /// if it isn't.
    fn is_satisfied(&self, idx: Self::TxnId, key: &Self::Key) -> bool;

    /// Checks whether all the requests are satisfied without adding them as pending requests
    /// if they aren't.
    fn are_all_satisfied<'a, KS>(&self, idx: Self::TxnId, keys: KS) -> bool
    where
        Self::Key: 'a,
        KS: IntoIterator<Item = &'a Self::Key>,
    {
        keys.into_iter().all(|key| self.is_satisfied(idx, key))
    }
}

/// For each key, maintains a set of reservations and a set of pending requests,
/// each identified by a transaction ID.
/// A request for a key is satisfied if there are no reservations with smaller `TxnId`.
/// When a reservation is removed, returns the set of satisfied requests and removes them
/// from the reservation table.
///
/// The table can be in three phases:
/// making reservations, removing reservations, and making requests.
/// The disjointness between the phases is ensured automatically by the borrow checker.
/// Hence, the implementation can assume no concurrent operations from other phases.
pub trait ParallelReservationTable {
    type Key;
    type TxnId: Copy;

    type MakeReservationsPhase<'a>: MakeReservationsPhaseTrait<Key = Self::Key, TxnId = Self::TxnId>
    where
        Self: 'a;

    type RemoveReservationsPhase<'a>: RemoveReservationsPhaseTrait<Key = Self::Key, TxnId = Self::TxnId>
    where
        Self: 'a;

    type RequestsPhase<'a>: RequestsPhaseTrait<Key = Self::Key, TxnId = Self::TxnId>
    where
        Self: 'a;

    /// Returns a handle that allows adding reservations to the table.
    fn make_reservations_phase(&mut self) -> Self::MakeReservationsPhase<'_>;

    /// Returns a handle that allows removing reservations from the table.
    fn remove_reservations_phase(&mut self) -> Self::RemoveReservationsPhase<'_>;

    /// Returns a handle that allows making requests to the table.
    fn requests_phase(&mut self) -> Self::RequestsPhase<'_>;
}

struct TableEntry<Id> {
    reservations: MinHeapWithRemove<Id>,
    requests: MinHeap<Id>,
}

impl<Id: Ord> Default for TableEntry<Id> {
    fn default() -> Self {
        Self {
            reservations: Default::default(),
            requests: Default::default(),
        }
    }
}

/// A parallel reservation table implemented on top of [`DashMap`].
///
/// This is the simplest yet, perhaps, not the most efficient, implementation of
/// a parallel reservation table.
pub struct DashMapReservationTable<K, Id> {
    table: DashMap<K, TableEntry<Id>>,
}

impl<K, Id> Default for DashMapReservationTable<K, Id>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self {
            table: DashMap::new(),
        }
    }
}

impl<K, Id> ParallelReservationTable for DashMapReservationTable<K, Id>
where
    K: Hash + Eq + Clone,
    Id: Ord + Eq + Hash + Default + Copy,
{
    type Key = K;
    type RequestsPhase<'a> = DashMapRequestsPhase<'a, K, Id> where Self: 'a;
    type MakeReservationsPhase<'a> = DashMapMakeReservationsPhase<'a, K, Id> where Self: 'a;
    type RemoveReservationsPhase<'a> = DashMapRemoveReservationsPhase<'a, K, Id> where Self: 'a;
    type TxnId = Id;

    fn make_reservations_phase(&mut self) -> Self::MakeReservationsPhase<'_> {
        DashMapMakeReservationsPhase { inner: self }
    }

    fn remove_reservations_phase(&mut self) -> Self::RemoveReservationsPhase<'_> {
        DashMapRemoveReservationsPhase { inner: self }
    }

    fn requests_phase(&mut self) -> Self::RequestsPhase<'_> {
        DashMapRequestsPhase { inner: self }
    }
}

pub struct DashMapMakeReservationsPhase<'a, K, Id> {
    inner: &'a mut DashMapReservationTable<K, Id>,
}

impl<'a, K, Id> MakeReservationsPhaseTrait for DashMapMakeReservationsPhase<'a, K, Id>
where
    K: Hash + Eq + Clone,
    Id: Ord + Eq + Hash + Default + Copy,
{
    type Key = K;
    type TxnId = Id;

    fn make_reservation(&self, idx: Id, key: &K) {
        self
            .inner
            .table
            .entry(key.clone())
            .or_default()
            .reservations
            .push(idx);
    }
}

pub struct DashMapRemoveReservationsPhase<'a, K, Id> {
    inner: &'a mut DashMapReservationTable<K, Id>,
}

impl<'a, K, Id> RemoveReservationsPhaseTrait for DashMapRemoveReservationsPhase<'a, K, Id>
where
    K: Hash + Eq + Clone,
    Id: Ord + Eq + Hash + Default + Copy,
{
    type Key = K;
    type TxnId = Id;

    fn remove_reservation(&self, idx: Id, key: &K) -> Vec<Id> {
        let mut entry = self.inner.table.get_mut(key).unwrap();
        entry.reservations.remove(idx);

        match entry.reservations.peek() {
            None => {
                let mut res = Default::default();
                std::mem::swap(&mut res, &mut entry.requests);
                res.into_iter().collect()
            },
            Some(&first_reservation) => {
                let mut res = Vec::new();

                while let Some(&first_request) = entry.requests.peek() {
                    if first_request > first_reservation {
                        break;
                    }
                    res.push(entry.requests.pop().unwrap());
                }

                res
            },
        }
    }
}

pub struct DashMapRequestsPhase<'a, K, Id> {
    inner: &'a mut DashMapReservationTable<K, Id>,
}

impl<'a, K, Id> RequestsPhaseTrait for DashMapRequestsPhase<'a, K, Id>
where
    K: Hash + Eq + Clone,
    Id: Ord + Eq + Hash + Default + Copy,
{
    type Key = K;
    type TxnId = Id;

    fn make_request(&self, idx: Id, key: &K) -> bool {
        if !self.is_satisfied(idx, key) {
            let requests = &mut self.inner.table.get_mut(key).unwrap().requests;
            requests.push(idx);
            true
        } else {
            false
        }
    }

    fn is_satisfied(&self, idx: Id, request_key: &K) -> bool {
        let Some(entry) = self.inner.table.get(request_key) else { return true };
        let Some(&first_reservation) = entry.reservations.peek() else { return true };
        first_reservation >= idx
    }
}
