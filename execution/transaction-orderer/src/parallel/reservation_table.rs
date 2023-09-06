// Copyright © Aptos Foundation

use std::{
    collections::{HashSet, VecDeque},
    hash::Hash,
};

use dashmap::DashMap;

pub trait ReservationsPhaseTrait {
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
/// The table can be in two phases: reservations phase and requests phase.
/// The disjointness between the two is ensured automatically by the borrow checker.
pub trait ParallelReservationTable {
    type Key;
    type TxnId: Copy;

    type ReservationsPhase<'a>: ReservationsPhaseTrait<Key = Self::Key, TxnId = Self::TxnId>
    where
        Self: 'a;
    type RequestsPhase<'a>: RequestsPhaseTrait<Key = Self::Key, TxnId = Self::TxnId>
    where
        Self: 'a;

    /// Returns a handle that allows adding and removing reservations from the table.
    /// While the handle is held, no requests can be added to the table.
    fn reservations_phase(&mut self) -> Self::ReservationsPhase<'_>;

    /// Returns a handle that allows adding requests to the table.
    /// While the handle is held, reservations cannot be added or removed.
    fn requests_phase(&mut self) -> Self::RequestsPhase<'_>;
}

#[derive(Default)]
struct TableEntry<Id> {
    reservations: VecDeque<Id>,
    requests: VecDeque<Id>,
    delayed_removes: HashSet<Id>,
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
    type ReservationsPhase<'a> = DashMapReservationsPhase<'a, K, Id> where Self: 'a;
    type TxnId = Id;

    fn reservations_phase(&mut self) -> Self::ReservationsPhase<'_> {
        DashMapReservationsPhase { inner: self }
    }

    fn requests_phase(&mut self) -> Self::RequestsPhase<'_> {
        DashMapRequestsPhase { inner: self }
    }
}

pub struct DashMapReservationsPhase<'a, K, Id> {
    inner: &'a mut DashMapReservationTable<K, Id>,
}

impl<'a, K, Id> ReservationsPhaseTrait for DashMapReservationsPhase<'a, K, Id>
where
    K: Hash + Eq + Clone,
    Id: Ord + Eq + Hash + Default + Copy,
{
    type Key = K;
    type TxnId = Id;

    fn make_reservation(&self, idx: Id, key: &K) {
        let reservations = &mut self
            .inner
            .table
            .entry(key.clone())
            .or_default()
            .reservations;
        debug_assert!(reservations.back().map_or(true, |&last| last <= idx));
        reservations.push_back(idx);
    }

    fn remove_reservation(&self, idx: Id, key: &K) -> Vec<Id> {
        let mut entry = self.inner.table.get_mut(key).unwrap();
        entry.delayed_removes.insert(idx);

        while let Some(&first_reservation) = entry.reservations.front() {
            if !entry.delayed_removes.contains(&first_reservation) {
                break;
            }
            entry.delayed_removes.remove(&first_reservation);
            entry.reservations.pop_front();
        }

        match entry.reservations.front() {
            None => {
                let mut res = VecDeque::new();
                std::mem::swap(&mut res, &mut entry.requests);
                res.into()
            },
            Some(&first_reservation) => {
                let mut res = Vec::new();

                while let Some(&first_request) = entry.requests.front() {
                    if first_request > first_reservation {
                        break;
                    }
                    res.push(entry.requests.pop_front().unwrap());
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
            debug_assert!(requests.back().map_or(true, |&last| last <= idx));
            self.inner
                .table
                .get_mut(key)
                .unwrap()
                .requests
                .push_back(idx);
            true
        } else {
            false
        }
    }

    fn is_satisfied(&self, idx: Id, request_key: &K) -> bool {
        let Some(entry) = self.inner.table.get(request_key) else { return true };
        let Some(&first_reservation) = entry.reservations.front() else { return true };
        first_reservation >= idx
    }
}
