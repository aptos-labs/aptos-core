// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

/// This module provides various indexes used by Mempool.
use crate::{
    core_mempool::transaction::{MempoolTransaction, TimelineState},
    counters,
    logging::{LogEntry, LogSchema},
    shared_mempool::types::{MultiBucketTimelineIndexIds, TimelineIndexIdentifier},
};
use aptos_consensus_types::common::TransactionSummary;
use aptos_crypto::HashValue;
use aptos_logger::error;
use aptos_types::{account_address::AccountAddress, transaction::ReplayProtector};
use rand::seq::SliceRandom;
use std::{
    cmp::Ordering,
    collections::{btree_map::RangeMut, btree_set::Iter, BTreeMap, BTreeSet, HashMap},
    hash::Hash,
    iter::Rev,
    mem,
    ops::{Bound, RangeBounds},
    time::{Duration, Instant, SystemTime},
};

#[derive(Clone, Default)]
pub struct AccountTransactions {
    nonce_transactions: BTreeMap<u64 /* Nonce */, MempoolTransaction>,
    sequence_number_transactions: BTreeMap<u64 /* Sequence number */, MempoolTransaction>,
}

impl AccountTransactions {
    pub(crate) fn get(&self, replay_protector: &ReplayProtector) -> Option<&MempoolTransaction> {
        match replay_protector {
            ReplayProtector::Nonce(nonce) => self.nonce_transactions.get(nonce),
            ReplayProtector::SequenceNumber(sequence_number) => {
                self.sequence_number_transactions.get(sequence_number)
            },
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        replay_protector: &ReplayProtector,
    ) -> Option<&mut MempoolTransaction> {
        match replay_protector {
            ReplayProtector::Nonce(nonce) => self.nonce_transactions.get_mut(nonce),
            ReplayProtector::SequenceNumber(sequence_number) => {
                self.sequence_number_transactions.get_mut(sequence_number)
            },
        }
    }

    pub(crate) fn insert(&mut self, txn: MempoolTransaction) {
        match txn.get_replay_protector() {
            ReplayProtector::Nonce(nonce) => {
                self.nonce_transactions.insert(nonce, txn);
            },
            ReplayProtector::SequenceNumber(sequence_number) => {
                self.sequence_number_transactions
                    .insert(sequence_number, txn);
            },
        }
    }

    pub(crate) fn remove(
        &mut self,
        replay_protector: &ReplayProtector,
    ) -> Option<MempoolTransaction> {
        match replay_protector {
            ReplayProtector::Nonce(nonce) => self.nonce_transactions.remove(nonce),
            ReplayProtector::SequenceNumber(sequence_number) => {
                self.sequence_number_transactions.remove(sequence_number)
            },
        }
    }

    pub(crate) fn append(&mut self, other: &mut Self) {
        self.nonce_transactions
            .append(&mut other.nonce_transactions);
        self.sequence_number_transactions
            .append(&mut other.sequence_number_transactions);
    }

    pub(crate) fn clear(&mut self) {
        self.nonce_transactions.clear();
        self.sequence_number_transactions.clear();
    }

    pub(crate) fn seq_num_split_off(&mut self, sequence_number: u64) -> Self {
        AccountTransactions {
            sequence_number_transactions: self
                .sequence_number_transactions
                .split_off(&sequence_number),
            nonce_transactions: mem::take(&mut self.nonce_transactions),
        }
    }

    pub(crate) fn seq_num_range_mut(
        &mut self,
        range: impl RangeBounds<u64>,
    ) -> RangeMut<'_, u64, MempoolTransaction> {
        self.sequence_number_transactions.range_mut(range)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &MempoolTransaction> {
        self.nonce_transactions
            .values()
            .chain(self.sequence_number_transactions.values())
    }

    pub(crate) fn orderless_txns_len(&self) -> usize {
        self.nonce_transactions.len()
    }

    pub(crate) fn seq_num_txns_len(&self) -> usize {
        self.sequence_number_transactions.len()
    }

    pub(crate) fn len(&self) -> usize {
        self.nonce_transactions.len() + self.sequence_number_transactions.len()
    }
}

/// PriorityIndex represents the main Priority Queue in Mempool.
/// It's used to form the transaction block for Consensus.
/// Transactions are ordered by gas price. Second level ordering is done by expiration time.
///
/// We don't store the full content of transactions in the index.
/// Instead we use `OrderedQueueKey` - logical reference to the transaction in the main store.
pub struct PriorityIndex {
    data: BTreeSet<OrderedQueueKey>,
}

pub type PriorityQueueIter<'a> = Rev<Iter<'a, OrderedQueueKey>>;

impl PriorityIndex {
    pub(crate) fn new() -> Self {
        Self {
            data: BTreeSet::new(),
        }
    }

    pub(crate) fn insert(&mut self, txn: &MempoolTransaction) -> bool {
        self.data.insert(self.make_key(txn))
    }

    pub(crate) fn remove(&mut self, txn: &MempoolTransaction) {
        self.data.remove(&self.make_key(txn));
    }

    pub(crate) fn contains(&self, txn: &MempoolTransaction) -> bool {
        self.data.contains(&self.make_key(txn))
    }

    fn make_key(&self, txn: &MempoolTransaction) -> OrderedQueueKey {
        OrderedQueueKey {
            gas_ranking_score: txn.ranking_score,
            expiration_time: txn.expiration_time,
            insertion_time: txn.insertion_info.insertion_time,
            address: txn.get_sender(),
            replay_protector: txn.get_replay_protector(),
            hash: txn.get_committed_hash(),
        }
    }

    pub(crate) fn iter(&self) -> PriorityQueueIter {
        self.data.iter().rev()
    }

    pub(crate) fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct OrderedQueueKey {
    pub gas_ranking_score: u64,
    pub expiration_time: Duration,
    pub insertion_time: SystemTime,
    pub address: AccountAddress,
    pub replay_protector: ReplayProtector,
    pub hash: HashValue,
}

impl PartialOrd for OrderedQueueKey {
    fn partial_cmp(&self, other: &OrderedQueueKey) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedQueueKey {
    fn cmp(&self, other: &OrderedQueueKey) -> Ordering {
        // Higher gas preferred
        match self.gas_ranking_score.cmp(&other.gas_ranking_score) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        // Lower insertion time preferred
        match self.insertion_time.cmp(&other.insertion_time).reverse() {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        // Higher address preferred
        match self.address.cmp(&other.address) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        // Question[Orderless]: Orderless transactions with Nonce are always prioritized over regular sequence number transactions.
        // Is it okay?
        match self.replay_protector.cmp(&other.replay_protector).reverse() {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        self.hash.cmp(&other.hash)
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
        // Ideally, we should garbage collect all transactions with expiration time < now.
        let max_expiration_time = now.saturating_sub(Duration::from_micros(1));
        let ttl_key = TTLOrderingKey {
            expiration_time: max_expiration_time,
            address: AccountAddress::ZERO,
            replay_protector: ReplayProtector::Nonce(0),
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
            replay_protector: txn.get_replay_protector(),
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
    pub replay_protector: ReplayProtector,
}

/// Be very careful with this, to not break the partial ordering.
/// See:  https://rust-lang.github.io/rust-clippy/master/index.html#derive_ord_xor_partial_ord
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for TTLOrderingKey {
    fn cmp(&self, other: &TTLOrderingKey) -> Ordering {
        match self.expiration_time.cmp(&other.expiration_time) {
            Ordering::Equal => match self.address.cmp(&other.address) {
                Ordering::Equal => self.replay_protector.cmp(&other.replay_protector),
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
}

/// TimelineId is the unique id of a transaction inserted into a timeline.
/// It's an auto incrementing counter.
pub type TimelineId = u64;

/// TimelineIndex is an ordered log of all transactions that are "ready" for mempool broadcast.
/// We only add a transaction to the index if it has a chance to be included in the next consensus
/// block (which means its status is != NotReady or its sequential to another "ready" transaction).
///
/// It's represented as Map <timeline_id, (Address, Replay Protector)>, where timeline_id is auto
/// increment unique id of "ready" transaction in local Mempool. (Address, Replay Protector) is a
/// logical reference to transaction content in main storage.
pub struct TimelineIndex {
    // Every transaction inserted into the TimelineIndex gets a unique timeline id.
    // This id is an auto incrementing counter.
    next_timeline_id: TimelineId,
    timeline: BTreeMap<TimelineId, (AccountAddress, ReplayProtector, Instant)>,
}

impl TimelineIndex {
    pub(crate) fn new() -> Self {
        Self {
            next_timeline_id: 1,
            timeline: BTreeMap::new(),
        }
    }

    /// Read all transactions from the timeline since <timeline_id>.
    /// At most `count` transactions will be returned.
    /// If `before` is set, only transactions inserted before this time will be returned.
    pub(crate) fn read_timeline(
        &self,
        timeline_id: TimelineId,
        count: usize,
        before: Option<Instant>,
    ) -> Vec<(AccountAddress, ReplayProtector)> {
        let mut batch = vec![];
        for (_id, &(address, replay_protector, insertion_time)) in self
            .timeline
            .range((Bound::Excluded(timeline_id), Bound::Unbounded))
        {
            if let Some(before) = before {
                if insertion_time >= before {
                    break;
                }
            }
            if batch.len() == count {
                break;
            }
            batch.push((address, replay_protector));
        }
        batch
    }

    /// Read transactions from the timeline from `start_timeline_id` (exclusive) to `end_timeline_id` (inclusive).
    pub(crate) fn timeline_range(
        &self,
        start_timeline_id: TimelineId,
        end_timeline_id: TimelineId,
    ) -> Vec<(AccountAddress, ReplayProtector)> {
        self.timeline
            .range((
                Bound::Excluded(start_timeline_id),
                Bound::Included(end_timeline_id),
            ))
            .map(|(_idx, &(address, replay_protector, _))| (address, replay_protector))
            .collect()
    }

    pub(crate) fn insert(&mut self, txn: &mut MempoolTransaction) {
        self.timeline.insert(
            self.next_timeline_id,
            (txn.get_sender(), txn.get_replay_protector(), Instant::now()),
        );
        txn.timeline_state = TimelineState::Ready(self.next_timeline_id);
        self.next_timeline_id += 1;
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

/// We use ranking score as a means to prioritize transactions.
/// At the moment, we use gas_unit_price in the transaction as ranking score.
/// Transactions with higher ranking score (gas_unit_price) are given higher priority.
type RankingScore = u64;

/// We divide the transactions into multiple buckets based on the ranking score.
/// Transactions with ranking score between bucket_mins[i] and bucket_mins[i+1] are stored in the ith bucket.
pub struct MultiBucketTimelineIndex {
    timelines: Vec<TimelineIndex>,
    bucket_mins: Vec<RankingScore>,
    bucket_mins_to_string: Vec<String>,
}

impl MultiBucketTimelineIndex {
    pub(crate) fn new(bucket_mins: Vec<RankingScore>) -> anyhow::Result<Self> {
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
        multibucket_timeline_ids: &MultiBucketTimelineIndexIds,
        count: usize,
        before: Option<Instant>,
    ) -> Vec<Vec<(AccountAddress, ReplayProtector)>> {
        assert!(multibucket_timeline_ids.id_per_bucket.len() == self.bucket_mins.len());

        let mut added = 0;
        let mut returned = vec![];
        for (timeline, &timeline_id) in self
            .timelines
            .iter()
            .zip(multibucket_timeline_ids.id_per_bucket.iter())
            .rev()
        {
            let txns = timeline.read_timeline(timeline_id, count - added, before);
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

    /// Read transactions from the timeline from `start_timeline_id` (exclusive) to `end_timeline_id` (inclusive).
    pub(crate) fn timeline_range(
        &self,
        start_end_pairs: HashMap<TimelineIndexIdentifier, (TimelineId, TimelineId)>,
    ) -> Vec<(AccountAddress, ReplayProtector)> {
        assert_eq!(start_end_pairs.len(), self.timelines.len());

        let mut all_txns = vec![];
        for (timeline_index_identifier, (start_id, end_id)) in start_end_pairs {
            let mut txns = self
                .timelines
                .get(timeline_index_identifier as usize)
                .map_or_else(Vec::new, |timeline| {
                    timeline.timeline_range(start_id, end_id)
                });
            all_txns.append(&mut txns);
        }
        all_txns
    }

    #[inline]
    fn get_timeline(&mut self, ranking_score: RankingScore) -> &mut TimelineIndex {
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
    pub(crate) fn get_bucket(&self, ranking_score: RankingScore) -> &str {
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
    data: Vec<(AccountAddress, BTreeSet<(u64, HashValue)>)>,
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

    pub(crate) fn insert(&mut self, txn: &mut MempoolTransaction) {
        // Orderless transactions are always in the "ready" state and are not stored in the parking lot.
        match txn.get_replay_protector() {
            ReplayProtector::SequenceNumber(sequence_number) => {
                if txn.insertion_info.park_time.is_none() {
                    txn.insertion_info.park_time = Some(SystemTime::now());
                }
                txn.was_parked = true;

                let sender = &txn.txn.sender();
                let hash = txn.get_committed_hash();
                let is_new_entry = match self.account_indices.get(sender) {
                    Some(index) => {
                        if let Some((_account, seq_nums)) = self.data.get_mut(*index) {
                            seq_nums.insert((sequence_number, hash))
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
                        let entry = [(sequence_number, hash)]
                            .iter()
                            .cloned()
                            .collect::<BTreeSet<_>>();
                        self.data.push((*sender, entry));
                        self.account_indices.insert(*sender, self.data.len() - 1);
                        true
                    },
                };
                if is_new_entry {
                    self.size += 1;
                }
            },
            ReplayProtector::Nonce(_) => {},
        }
    }

    pub(crate) fn remove(&mut self, txn: &MempoolTransaction) {
        // Orderless transactions are always in the "ready" state and are not stored in the parking lot.
        match txn.get_replay_protector() {
            ReplayProtector::SequenceNumber(sequence_number) => {
                let sender = &txn.txn.sender();
                if let Some(index) = self.account_indices.get(sender).cloned() {
                    if let Some((_account, txns)) = self.data.get_mut(index) {
                        if txns.remove(&(sequence_number, txn.get_committed_hash())) {
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
            },
            ReplayProtector::Nonce(_) => {},
        }
    }

    pub(crate) fn contains(
        &self,
        account: &AccountAddress,
        replay_protector: ReplayProtector,
        hash: HashValue,
    ) -> bool {
        // Orderless transactions are always in the "ready" state and are not stored in the parking lot.
        match replay_protector {
            ReplayProtector::SequenceNumber(seq_num) => self
                .account_indices
                .get(account)
                .and_then(|idx| self.data.get(*idx))
                .map_or(false, |(_account, txns)| txns.contains(&(seq_num, hash))),
            ReplayProtector::Nonce(_) => false,
        }
    }

    /// Returns a random "non-ready" transaction (with highest sequence number for that account).
    pub(crate) fn get_poppable(&self) -> Option<TxnPointer> {
        let mut rng = rand::thread_rng();
        self.data.choose(&mut rng).and_then(|(sender, txns)| {
            txns.iter().next_back().map(|(seq_num, hash)| TxnPointer {
                sender: *sender,
                replay_protector: ReplayProtector::SequenceNumber(*seq_num),
                hash: *hash,
            })
        })
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn get_addresses(&self) -> Vec<(AccountAddress, u64)> {
        self.data
            .iter()
            .map(|(addr, txns)| (*addr, txns.len() as u64))
            .collect::<Vec<(AccountAddress, u64)>>()
    }
}

/// Logical pointer to `MempoolTransaction`.
/// Includes Account's address and transaction sequence number.
pub type TxnPointer = TransactionSummary;

impl From<&MempoolTransaction> for TxnPointer {
    fn from(txn: &MempoolTransaction) -> Self {
        Self {
            sender: txn.get_sender(),
            replay_protector: txn.get_replay_protector(),
            hash: txn.get_committed_hash(),
        }
    }
}

impl From<&OrderedQueueKey> for TxnPointer {
    fn from(key: &OrderedQueueKey) -> Self {
        Self {
            sender: key.address,
            replay_protector: key.replay_protector,
            hash: key.hash,
        }
    }
}
