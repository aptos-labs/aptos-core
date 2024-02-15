// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

/// This module provides various indexes used by Mempool.
use crate::core_mempool::transaction::{MempoolTransaction, SequenceInfo, TimelineState};
use crate::{
    counters,
    logging::{LogEntry, LogSchema},
    shared_mempool::types::MultiBucketTimelineIndexIds,
};
use aptos_consensus_types::common::TransactionSummary;
use aptos_logger::prelude::*;
use aptos_types::account_address::AccountAddress;
use rand::seq::SliceRandom;
use std::{
    cmp::Ordering,
    collections::{btree_set::Iter, BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    ops::Bound,
    time::Duration,
};

pub type AccountTransactions = BTreeMap<u64, MempoolTransaction>;

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct OrderedQueueKey {
    pub gas_ranking_score: u64,
    pub expiration_time: Duration,
    pub address: AccountAddress,
    pub sequence_number: SequenceInfo,
}

impl OrderedQueueKey {
    fn make_key(txn: &MempoolTransaction) -> OrderedQueueKey {
        OrderedQueueKey {
            gas_ranking_score: txn.ranking_score,
            expiration_time: txn.expiration_time,
            address: txn.get_sender(),
            sequence_number: txn.sequence_info,
        }
    }
}

impl PartialOrd for OrderedQueueKey {
    fn partial_cmp(&self, other: &OrderedQueueKey) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedQueueKey {
    fn cmp(&self, other: &OrderedQueueKey) -> Ordering {
        match self.gas_ranking_score.cmp(&other.gas_ranking_score) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        match self.expiration_time.cmp(&other.expiration_time).reverse() {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        match self.address.cmp(&other.address) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        self.sequence_number
            .transaction_sequence_number
            .cmp(&other.sequence_number.transaction_sequence_number)
            .reverse()
    }
}

/// TTLIndex is used to perform garbage collection of old transactions in Mempool.
/// Periodically separate GC-like job queries this index to find out transactions that have to be
/// removed. Index is represented as `BTreeSet<TTLOrderingKey>`, where `TTLOrderingKey`
/// is a logical reference to TxnInfo.
/// Index is ordered by `TTLOrderingKey::expiration_time`.
pub struct TTLIndex {
    data: BTreeSet<TTLOrderingKey>,
    get_expiration_time: Box<dyn Fn(&MempoolTransaction) -> Duration + Send + Sync>,
}

impl TTLIndex {
    pub(crate) fn new<F>(get_expiration_time: Box<F>) -> Self
    where
        F: Fn(&MempoolTransaction) -> Duration + 'static + Send + Sync,
    {
        Self {
            data: BTreeSet::new(),
            get_expiration_time,
        }
    }

    pub(crate) fn insert(&mut self, txn: &MempoolTransaction) {
        self.data.insert(self.make_key(txn));
    }

    pub(crate) fn remove(&mut self, txn: &MempoolTransaction) {
        self.data.remove(&self.make_key(txn));
    }

    /// Garbage collect all old transactions.
    pub(crate) fn gc(&mut self, now: Duration) -> Vec<TTLOrderingKey> {
        let ttl_key = TTLOrderingKey {
            expiration_time: now,
            address: AccountAddress::ZERO,
            sequence_number: 0,
        };

        let mut active = self.data.split_off(&ttl_key);
        let ttl_transactions = self.data.iter().cloned().collect();
        self.data.clear();
        self.data.append(&mut active);
        ttl_transactions
    }

    fn make_key(&self, txn: &MempoolTransaction) -> TTLOrderingKey {
        TTLOrderingKey {
            expiration_time: (self.get_expiration_time)(txn),
            address: txn.get_sender(),
            sequence_number: txn.sequence_info.transaction_sequence_number,
        }
    }

    pub(crate) fn iter(&self) -> Iter<TTLOrderingKey> {
        self.data.iter()
    }

    pub(crate) fn size(&self) -> usize {
        self.data.len()
    }
}

#[allow(clippy::derive_ord_xor_partial_ord)]
#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub struct TTLOrderingKey {
    pub expiration_time: Duration,
    pub address: AccountAddress,
    pub sequence_number: u64,
}

/// Be very careful with this, to not break the partial ordering.
/// See:  https://rust-lang.github.io/rust-clippy/master/index.html#derive_ord_xor_partial_ord
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for TTLOrderingKey {
    fn cmp(&self, other: &TTLOrderingKey) -> Ordering {
        match self.expiration_time.cmp(&other.expiration_time) {
            Ordering::Equal => {
                (&self.address, self.sequence_number).cmp(&(&other.address, other.sequence_number))
            },
            ordering => ordering,
        }
    }
}

/// TimelineIndex is an ordered log of all transactions that are "ready" for broadcast.
/// We only add a transaction to the index if it has a chance to be included in the next consensus
/// block (which means its status is != NotReady or its sequential to another "ready" transaction).
///
/// It's represented as Map <timeline_id, (Address, sequence_number)>, where timeline_id is auto
/// increment unique id of "ready" transaction in local Mempool. (Address, sequence_number) is a
/// logical reference to transaction content in main storage.
pub struct TimelineIndex {
    timeline_id: u64,
    timeline: BTreeMap<u64, (AccountAddress, u64)>,
}

impl TimelineIndex {
    pub(crate) fn new() -> Self {
        Self {
            timeline_id: 1,
            timeline: BTreeMap::new(),
        }
    }

    /// Read all transactions from the timeline since <timeline_id>.
    /// At most `count` transactions will be returned.
    pub(crate) fn read_timeline(
        &self,
        timeline_id: u64,
        count: usize,
    ) -> Vec<(AccountAddress, u64)> {
        let mut batch = vec![];
        for (_id, &(address, sequence_number)) in self
            .timeline
            .range((Bound::Excluded(timeline_id), Bound::Unbounded))
        {
            batch.push((address, sequence_number));
            if batch.len() == count {
                break;
            }
        }
        batch
    }

    /// Read transactions from the timeline from `start_id` (exclusive) to `end_id` (inclusive).
    pub(crate) fn timeline_range(&self, start_id: u64, end_id: u64) -> Vec<(AccountAddress, u64)> {
        self.timeline
            .range((Bound::Excluded(start_id), Bound::Included(end_id)))
            .map(|(_idx, txn)| txn)
            .cloned()
            .collect()
    }

    pub(crate) fn insert(&mut self, txn: &mut MempoolTransaction) {
        self.timeline.insert(
            self.timeline_id,
            (
                txn.get_sender(),
                txn.sequence_info.transaction_sequence_number,
            ),
        );
        txn.timeline_state = TimelineState::Ready(self.timeline_id);
        self.timeline_id += 1;
    }

    pub(crate) fn remove(&mut self, txn: &MempoolTransaction) {
        if let TimelineState::Ready(timeline_id) = txn.timeline_state {
            self.timeline.remove(&timeline_id);
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.timeline.len()
    }
}

pub struct MultiBucketTimelineIndex {
    timelines: Vec<TimelineIndex>,
    bucket_mins: Vec<u64>,
    bucket_mins_to_string: Vec<String>,
}

impl MultiBucketTimelineIndex {
    pub(crate) fn new(bucket_mins: Vec<u64>) -> anyhow::Result<Self> {
        anyhow::ensure!(!bucket_mins.is_empty(), "Must not be empty");
        anyhow::ensure!(bucket_mins[0] == 0, "First bucket must start at 0");

        let mut prev = None;
        let mut timelines = vec![];
        for entry in bucket_mins.clone() {
            if let Some(prev) = prev {
                anyhow::ensure!(prev < entry, "Values must be sorted and not repeat");
            }
            prev = Some(entry);
            timelines.push(TimelineIndex::new());
        }

        let bucket_mins_to_string: Vec<_> = bucket_mins
            .iter()
            .map(|bucket_min| bucket_min.to_string())
            .collect();

        Ok(Self {
            timelines,
            bucket_mins,
            bucket_mins_to_string,
        })
    }

    /// Read all transactions from the timeline since <timeline_id>.
    /// At most `count` transactions will be returned.
    pub(crate) fn read_timeline(
        &self,
        timeline_id: &MultiBucketTimelineIndexIds,
        count: usize,
    ) -> Vec<Vec<(AccountAddress, u64)>> {
        assert!(timeline_id.id_per_bucket.len() == self.bucket_mins.len());

        let mut added = 0;
        let mut returned = vec![];
        for (timeline, &timeline_id) in self
            .timelines
            .iter()
            .zip(timeline_id.id_per_bucket.iter())
            .rev()
        {
            let txns = timeline.read_timeline(timeline_id, count - added);
            added += txns.len();
            returned.push(txns);

            if added == count {
                break;
            }
        }
        while returned.len() < self.timelines.len() {
            returned.push(vec![]);
        }
        returned.iter().rev().cloned().collect()
    }

    /// Read transactions from the timeline from `start_id` (exclusive) to `end_id` (inclusive).
    pub(crate) fn timeline_range(
        &self,
        start_end_pairs: &Vec<(u64, u64)>,
    ) -> Vec<(AccountAddress, u64)> {
        assert_eq!(start_end_pairs.len(), self.timelines.len());

        let mut all_txns = vec![];
        for (timeline, &(start_id, end_id)) in self.timelines.iter().zip(start_end_pairs.iter()) {
            let mut txns = timeline.timeline_range(start_id, end_id);
            all_txns.append(&mut txns);
        }
        all_txns
    }

    #[inline]
    fn get_timeline(&mut self, ranking_score: u64) -> &mut TimelineIndex {
        let index = self
            .bucket_mins
            .binary_search(&ranking_score)
            .unwrap_or_else(|i| i - 1);
        self.timelines.get_mut(index).unwrap()
    }

    pub(crate) fn insert(&mut self, txn: &mut MempoolTransaction) {
        self.get_timeline(txn.ranking_score).insert(txn);
    }

    pub(crate) fn remove(&mut self, txn: &MempoolTransaction) {
        self.get_timeline(txn.ranking_score).remove(txn);
    }

    pub(crate) fn size(&self) -> usize {
        let mut size = 0;
        for timeline in &self.timelines {
            size += timeline.size()
        }
        size
    }

    pub(crate) fn get_sizes(&self) -> Vec<(&str, usize)> {
        self.bucket_mins_to_string
            .iter()
            .zip(self.timelines.iter())
            .map(|(bucket_min, timeline)| (bucket_min.as_str(), timeline.size()))
            .collect()
    }

    #[inline]
    pub(crate) fn get_bucket(&self, ranking_score: u64) -> &str {
        let index = self
            .bucket_mins
            .binary_search(&ranking_score)
            .unwrap_or_else(|i| i - 1);
        self.bucket_mins_to_string[index].as_str()
    }
}

/// ParkingLotIndex keeps track of "not_ready" transactions, e.g., transactions that
/// can't be included in the next block because their sequence number is too high.
/// We keep a separate index to be able to efficiently evict them when Mempool is full.
pub struct ParkingLotIndex {
    // DS invariants:
    // 1. for each entry (account, txns) in `data`, `txns` is never empty
    // 2. for all accounts, data.get(account_indices.get(`account`)) == (account, sequence numbers of account's txns)
    data: Vec<(AccountAddress, BTreeSet<u64>)>,
    account_indices: HashMap<AccountAddress, usize>,
    size: usize,
}

impl ParkingLotIndex {
    pub(crate) fn new() -> Self {
        Self {
            data: vec![],
            account_indices: HashMap::new(),
            size: 0,
        }
    }

    pub(crate) fn insert(&mut self, txn: &MempoolTransaction) {
        let sender = &txn.txn.sender();
        let sequence_number = txn.txn.sequence_number();
        let is_new_entry = match self.account_indices.get(sender) {
            Some(index) => {
                if let Some((_account, seq_nums)) = self.data.get_mut(*index) {
                    seq_nums.insert(sequence_number)
                } else {
                    counters::CORE_MEMPOOL_INVARIANT_VIOLATION_COUNT.inc();
                    error!(
                        LogSchema::new(LogEntry::InvariantViolated),
                        "Parking lot invariant violated: for account {}, account index exists but missing entry in data",
                        sender
                    );
                    return;
                }
            },
            None => {
                let seq_nums = [sequence_number].iter().cloned().collect::<BTreeSet<_>>();
                self.data.push((*sender, seq_nums));
                self.account_indices.insert(*sender, self.data.len() - 1);
                true
            },
        };
        if is_new_entry {
            self.size += 1;
        }
    }

    pub(crate) fn remove(&mut self, txn: &MempoolTransaction) {
        let sender = &txn.txn.sender();
        if let Some(index) = self.account_indices.get(sender).cloned() {
            if let Some((_account, txns)) = self.data.get_mut(index) {
                if txns.remove(&txn.txn.sequence_number()) {
                    self.size -= 1;
                }

                // maintain DS invariant
                if txns.is_empty() {
                    // remove account with no more txns
                    self.data.swap_remove(index);
                    self.account_indices.remove(sender);

                    // update DS for account that was swapped in `swap_remove`
                    if let Some((swapped_account, _)) = self.data.get(index) {
                        self.account_indices.insert(*swapped_account, index);
                    }
                }
            }
        }
    }

    pub(crate) fn contains(&self, account: &AccountAddress, seq_num: &u64) -> bool {
        self.account_indices
            .get(account)
            .and_then(|idx| self.data.get(*idx))
            .map_or(false, |(_account, txns)| txns.contains(seq_num))
    }

    /// Returns a random "non-ready" transaction (with highest sequence number for that account).
    pub(crate) fn get_poppable(&self) -> Option<TxnPointer> {
        let mut rng = rand::thread_rng();
        self.data.choose(&mut rng).and_then(|(sender, txns)| {
            txns.iter().next_back().map(|seq_num| TxnPointer {
                sender: *sender,
                sequence_number: *seq_num,
            })
        })
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }
}

/// Logical pointer to `MempoolTransaction`.
/// Includes Account's address and transaction sequence number.
pub type TxnPointer = TransactionSummary;

impl From<&MempoolTransaction> for TxnPointer {
    fn from(txn: &MempoolTransaction) -> Self {
        Self {
            sender: txn.get_sender(),
            sequence_number: txn.sequence_info.transaction_sequence_number,
        }
    }
}

impl From<&OrderedQueueKey> for TxnPointer {
    fn from(key: &OrderedQueueKey) -> Self {
        Self {
            sender: key.address,
            sequence_number: key.sequence_number.transaction_sequence_number,
        }
    }
}

// TODO: A lazy GC. Also, keep in mind non-validators will never advance the index.
pub struct FifoIndex {
    ready: VecDeque<OrderedQueueKey>,
    previously_selected: Vec<OrderedQueueKey>,
    // TODO: wasteful, but maybe necessary
    all: HashSet<OrderedQueueKey>,
    previously_selected_idx: usize,
}

impl FifoIndex {
    pub(crate) fn new() -> Self {
        Self {
            ready: VecDeque::new(),
            previously_selected: vec![],
            all: HashSet::new(),
            previously_selected_idx: 0,
        }
    }

    /// Reset the previously_selected pointer
    pub fn reset(&mut self) {
        self.previously_selected_idx = 0;
    }

    // TODO: make a proper iterator?
    pub fn next(&mut self) -> Option<OrderedQueueKey> {
        while self.previously_selected_idx < self.previously_selected.len() {
            let txn = &self.previously_selected[self.previously_selected_idx];
            self.previously_selected_idx += 1;
            if self.all.contains(txn) {
                return Some(txn.clone());
            } else {
                self.previously_selected
                    .remove(self.previously_selected_idx - 1);
            }
        }
        while !self.ready.is_empty() {
            let next = self.ready.pop_front();
            if let Some(next) = next {
                if self.all.contains(&next) {
                    return Some(next);
                }
            }
        }
        None
    }

    pub fn insert(&mut self, txn: &MempoolTransaction) {
        let key = OrderedQueueKey::make_key(txn);
        self.ready.push_back(key.clone());
        self.all.insert(key);
    }

    pub fn contains(&self, txn: &MempoolTransaction) -> bool {
        self.all.contains(&OrderedQueueKey::make_key(txn))
    }

    pub(crate) fn size(&self) -> usize {
        self.all.len()
    }

    // TODO: This is lazy GC, so won't work for non-validators and validators that can't keep up
    pub fn remove(&mut self, txn: &MempoolTransaction) {
        self.all.remove(&OrderedQueueKey::make_key(txn));
    }
}

pub struct MultiBucketFifoIndex {
    indexes: Vec<FifoIndex>,
    bucket_mins: Vec<u64>,
    index_idx: usize,
}

impl MultiBucketFifoIndex {
    pub(crate) fn new(bucket_mins: Vec<u64>) -> anyhow::Result<Self> {
        anyhow::ensure!(!bucket_mins.is_empty(), "Must not be empty");
        anyhow::ensure!(bucket_mins[0] == 0, "First bucket must start at 0");

        let mut prev = None;
        let mut indexes = vec![];
        for entry in bucket_mins.clone() {
            if let Some(prev) = prev {
                anyhow::ensure!(prev < entry, "Values must be sorted and not repeat");
            }
            prev = Some(entry);
            indexes.push(FifoIndex::new());
        }

        Ok(Self {
            indexes,
            bucket_mins,
            index_idx: 0,
        })
    }

    pub fn reset(&mut self) {
        for index in &mut self.indexes {
            index.reset();
        }
    }

    pub fn next(&mut self) -> Option<OrderedQueueKey> {
        let mut next = None;
        for (i, index) in self.indexes.iter_mut().enumerate().skip(self.index_idx) {
            next = index.next();
            self.index_idx = i;
            if next.is_some() {
                break;
            }
        }
        next
    }

    fn get_index(&mut self, ranking_score: u64) -> &mut FifoIndex {
        let index = self
            .bucket_mins
            .binary_search(&ranking_score)
            .unwrap_or_else(|i| i - 1);
        self.indexes.get_mut(index).unwrap()
    }

    pub fn insert(&mut self, txn: &MempoolTransaction) {
        self.get_index(txn.ranking_score).insert(txn);
    }

    pub fn contains(&self, txn: &MempoolTransaction) -> bool {
        self.indexes.iter().any(|index| index.contains(txn))
    }

    pub fn size(&self) -> usize {
        let mut size = 0;
        for index in &self.indexes {
            size += index.size()
        }
        size
    }

    pub fn remove(&mut self, txn: &MempoolTransaction) {
        self.get_index(txn.ranking_score).remove(txn);
    }
}
