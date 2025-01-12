// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{
        index::{
            AccountTransactions, MultiBucketTimelineIndex, ParkingLotIndex, PriorityIndex,
            PriorityQueueIter, TTLIndex,
        },
        mempool::Mempool,
        transaction::{InsertionInfo, MempoolTransaction, TimelineState},
    },
    counters::{self, BROADCAST_BATCHED_LABEL, BROADCAST_READY_LABEL, CONSENSUS_READY_LABEL},
    logging::{LogEntry, LogEvent, LogSchema, TxnsLog},
    network::BroadcastPeerPriority,
    shared_mempool::types::{
        MempoolSenderBucket, MultiBucketTimelineIndexIds, TimelineIndexIdentifier,
    },
};
use aptos_config::config::MempoolConfig;
use aptos_crypto::HashValue;
use aptos_logger::{prelude::*, Level};
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::{MempoolStatus, MempoolStatusCode},
    transaction::SignedTransaction,
};
use std::{
    cmp::max,
    collections::HashMap,
    mem::size_of,
    ops::Bound,
    time::{Duration, Instant, SystemTime},
};

/// Estimated per-txn overhead of indexes. Needs to be updated if additional indexes are added.
pub const TXN_INDEX_ESTIMATED_BYTES: usize = size_of::<crate::core_mempool::index::OrderedQueueKey>() // priority_index
    + size_of::<crate::core_mempool::index::TTLOrderingKey>() * 2 // expiration_time_index + system_ttl_index
    + (size_of::<u64>() * 3 + size_of::<AccountAddress>()) // timeline_index
    + (size_of::<HashValue>() + size_of::<u64>() + size_of::<AccountAddress>()); // hash_index

pub fn sender_bucket(
    address: &AccountAddress,
    num_sender_buckets: MempoolSenderBucket,
) -> MempoolSenderBucket {
    address.as_ref()[address.as_ref().len() - 1] as MempoolSenderBucket % num_sender_buckets
}

/// TransactionStore is in-memory storage for all transactions in mempool.
pub struct TransactionStore {
    // main DS
    transactions: HashMap<AccountAddress, AccountTransactions>,

    // Sequence numbers for accounts with transactions
    sequence_numbers: HashMap<AccountAddress, u64>,

    // indexes
    priority_index: PriorityIndex,
    // TTLIndex based on client-specified expiration time
    expiration_time_index: TTLIndex,
    // TTLIndex based on system expiration time
    // we keep it separate from `expiration_time_index` so Mempool can't be clogged
    //  by old transactions even if it hasn't received commit callbacks for a while
    system_ttl_index: TTLIndex,
    // Broadcast-ready transactions.
    // For each sender bucket, we maintain a timeline per txn fee range.
    timeline_index: HashMap<MempoolSenderBucket, MultiBucketTimelineIndex>,
    // We divide the senders into buckets and maintain a separate set of timelines for each sender bucket.
    // This is the number of sender buckets.
    num_sender_buckets: MempoolSenderBucket,
    // keeps track of "non-ready" txns (transactions that can't be included in next block)
    parking_lot_index: ParkingLotIndex,
    // Index for looking up transaction by hash.
    // Transactions are stored by AccountAddress + sequence number.
    // This index stores map of transaction committed hash to (AccountAddress, sequence number) pair.
    // Using transaction commited hash because from end user's point view, a transaction should only have
    // one valid hash.
    hash_index: HashMap<HashValue, (AccountAddress, u64)>,
    // estimated size in bytes
    size_bytes: usize,

    // configuration
    capacity: usize,
    capacity_bytes: usize,
    capacity_per_user: usize,
    max_batch_bytes: u64,

    // eager expiration
    eager_expire_threshold: Option<Duration>,
    eager_expire_time: Duration,
}

impl TransactionStore {
    pub(crate) fn new(config: &MempoolConfig) -> Self {
        let mut timeline_index = HashMap::new();
        for sender_bucket in 0..config.num_sender_buckets {
            timeline_index.insert(
                sender_bucket,
                MultiBucketTimelineIndex::new(config.broadcast_buckets.clone()).unwrap(),
            );
        }
        Self {
            // main DS
            transactions: HashMap::new(),
            sequence_numbers: HashMap::new(),

            // various indexes
            system_ttl_index: TTLIndex::new(Box::new(|t: &MempoolTransaction| t.expiration_time)),
            expiration_time_index: TTLIndex::new(Box::new(|t: &MempoolTransaction| {
                Duration::from_secs(t.txn.expiration_timestamp_secs())
            })),
            priority_index: PriorityIndex::new(),
            timeline_index,
            num_sender_buckets: config.num_sender_buckets,
            parking_lot_index: ParkingLotIndex::new(),
            hash_index: HashMap::new(),
            // estimated size in bytes
            size_bytes: 0,

            // configuration
            capacity: config.capacity,
            capacity_bytes: config.capacity_bytes,
            capacity_per_user: config.capacity_per_user,
            max_batch_bytes: config.shared_mempool_max_batch_bytes,

            // eager expiration
            eager_expire_threshold: config.eager_expire_threshold_ms.map(Duration::from_millis),
            eager_expire_time: Duration::from_millis(config.eager_expire_time_ms),
        }
    }

    #[inline]
    fn get_mempool_txn(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<&MempoolTransaction> {
        self.transactions
            .get(address)
            .and_then(|txns| txns.get(&sequence_number))
    }

    /// Fetch transaction by account address + sequence_number.
    pub(crate) fn get(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<SignedTransaction> {
        if let Some(txn) = self.get_mempool_txn(address, sequence_number) {
            return Some(txn.txn.clone());
        }
        None
    }

    /// Fetch transaction by account address + sequence_number, including ranking score
    pub(crate) fn get_with_ranking_score(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<(SignedTransaction, u64)> {
        if let Some(txn) = self.get_mempool_txn(address, sequence_number) {
            return Some((txn.txn.clone(), txn.ranking_score));
        }
        None
    }

    pub(crate) fn get_by_hash(&self, hash: HashValue) -> Option<SignedTransaction> {
        match self.hash_index.get(&hash) {
            Some((address, seq)) => self.get(address, *seq),
            None => None,
        }
    }

    pub(crate) fn get_insertion_info_and_bucket(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<(&InsertionInfo, String, String)> {
        if let Some(txn) = self.get_mempool_txn(address, sequence_number) {
            return Some((
                &txn.insertion_info,
                self.get_bucket(txn.ranking_score, address),
                txn.priority_of_sender
                    .clone()
                    .map_or_else(|| "Unknown".to_string(), |priority| priority.to_string()),
            ));
        }
        None
    }

    pub(crate) fn get_ranking_score(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<u64> {
        if let Some(txn) = self.get_mempool_txn(address, sequence_number) {
            return Some(txn.ranking_score);
        }
        None
    }

    #[inline]
    pub(crate) fn get_bucket(&self, ranking_score: u64, sender: &AccountAddress) -> String {
        let sender_bucket = sender_bucket(sender, self.num_sender_buckets);
        let bucket = self
            .timeline_index
            .get(&sender_bucket)
            .unwrap()
            .get_bucket(ranking_score)
            .to_string();
        format!("{}_{}", sender_bucket, bucket)
    }

    pub(crate) fn get_sequence_number(&self, address: &AccountAddress) -> Option<&u64> {
        self.sequence_numbers.get(address)
    }

    pub(crate) fn num_sender_buckets(&self) -> MempoolSenderBucket {
        self.num_sender_buckets
    }

    /// Insert transaction into TransactionStore. Performs validation checks and updates indexes.
    pub(crate) fn insert(&mut self, txn: MempoolTransaction) -> MempoolStatus {
        let address = txn.get_sender();
        let txn_seq_num = txn.sequence_info.transaction_sequence_number;
        let acc_seq_num = txn.sequence_info.account_sequence_number;

        // If the transaction is already in Mempool, we only allow the user to
        // increase the gas unit price to speed up a transaction, but not the max gas.
        //
        // Transactions with all the same inputs (but possibly signed differently) are idempotent
        // since the raw transaction is the same
        if let Some(txns) = self.transactions.get_mut(&address) {
            if let Some(current_version) = txns.get_mut(&txn_seq_num) {
                if current_version.txn.payload() != txn.txn.payload() {
                    return MempoolStatus::new(MempoolStatusCode::InvalidUpdate).with_message(
                        "Transaction already in mempool with a different payload".to_string(),
                    );
                } else if current_version.txn.expiration_timestamp_secs()
                    != txn.txn.expiration_timestamp_secs()
                {
                    return MempoolStatus::new(MempoolStatusCode::InvalidUpdate).with_message(
                        "Transaction already in mempool with a different expiration timestamp"
                            .to_string(),
                    );
                } else if current_version.txn.max_gas_amount() != txn.txn.max_gas_amount() {
                    return MempoolStatus::new(MempoolStatusCode::InvalidUpdate).with_message(
                        "Transaction already in mempool with a different max gas amount"
                            .to_string(),
                    );
                } else if current_version.get_gas_price() < txn.get_gas_price() {
                    // Update txn if gas unit price is a larger value than before
                    if let Some(txn) = txns.remove(&txn_seq_num) {
                        self.index_remove(&txn);
                    };
                    counters::CORE_MEMPOOL_GAS_UPGRADED_TXNS.inc();
                } else if current_version.get_gas_price() > txn.get_gas_price() {
                    return MempoolStatus::new(MempoolStatusCode::InvalidUpdate).with_message(
                        "Transaction already in mempool with a higher gas price".to_string(),
                    );
                } else {
                    // If the transaction is the same, it's an idempotent call
                    // Updating signers is not supported, the previous submission must fail
                    counters::CORE_MEMPOOL_IDEMPOTENT_TXNS.inc();
                    return MempoolStatus::new(MempoolStatusCode::Accepted);
                }
            }
        }

        if self.check_is_full_after_eviction(&txn, acc_seq_num) {
            return MempoolStatus::new(MempoolStatusCode::MempoolIsFull).with_message(format!(
                "Mempool is full. Mempool size: {}, Capacity: {}",
                self.system_ttl_index.size(),
                self.capacity,
            ));
        }

        self.clean_committed_transactions(&address, acc_seq_num);

        self.transactions.entry(address).or_default();

        if let Some(txns) = self.transactions.get_mut(&address) {
            // capacity check
            if txns.len() >= self.capacity_per_user {
                return MempoolStatus::new(MempoolStatusCode::TooManyTransactions).with_message(
                    format!(
                        "Mempool over capacity for account. Number of transactions from account: {} Capacity per account: {}",
                        txns.len(),
                        self.capacity_per_user,
                    ),
                );
            }

            // insert into storage and other indexes
            self.system_ttl_index.insert(&txn);
            self.expiration_time_index.insert(&txn);
            self.hash_index
                .insert(txn.get_committed_hash(), (txn.get_sender(), txn_seq_num));
            self.sequence_numbers.insert(txn.get_sender(), acc_seq_num);
            self.size_bytes += txn.get_estimated_bytes();
            txns.insert(txn_seq_num, txn);
            self.track_indices();
        }
        self.process_ready_transactions(&address, acc_seq_num);
        MempoolStatus::new(MempoolStatusCode::Accepted)
    }

    fn track_indices(&self) {
        counters::core_mempool_index_size(
            counters::SYSTEM_TTL_INDEX_LABEL,
            self.system_ttl_index.size(),
        );
        counters::core_mempool_index_size(
            counters::EXPIRATION_TIME_INDEX_LABEL,
            self.expiration_time_index.size(),
        );
        counters::core_mempool_index_size(
            counters::PRIORITY_INDEX_LABEL,
            self.priority_index.size(),
        );
        counters::core_mempool_index_size(
            counters::PARKING_LOT_INDEX_LABEL,
            self.parking_lot_index.size(),
        );
        counters::core_mempool_index_size(
            counters::TIMELINE_INDEX_LABEL,
            self.timeline_index
                .values()
                .map(|timelines| timelines.size())
                .sum(),
        );

        let mut bucket_min_size_pairs = vec![];
        for (sender_bucket, timelines) in self.timeline_index.iter() {
            for (timeline_index_identifier, size) in timelines.get_sizes() {
                bucket_min_size_pairs.push((
                    format!("{}_{}", sender_bucket, timeline_index_identifier),
                    size,
                ));
            }
        }
        counters::core_mempool_timeline_index_size(bucket_min_size_pairs);
        counters::core_mempool_index_size(
            counters::TRANSACTION_HASH_INDEX_LABEL,
            self.hash_index.len(),
        );
        counters::core_mempool_index_size(counters::SIZE_BYTES_LABEL, self.size_bytes);
    }

    /// Checks if Mempool is full.
    /// If it's full, tries to free some space by evicting transactions from the ParkingLot.
    /// We only evict on attempt to insert a transaction that would be ready for broadcast upon insertion.
    fn check_is_full_after_eviction(
        &mut self,
        txn: &MempoolTransaction,
        curr_sequence_number: u64,
    ) -> bool {
        if self.is_full() && self.check_txn_ready(txn, curr_sequence_number) {
            let now = Instant::now();
            // try to free some space in Mempool from ParkingLot by evicting non-ready txns
            let mut evicted_txns = 0;
            let mut evicted_bytes = 0;
            while let Some(txn_pointer) = self.parking_lot_index.get_poppable() {
                if let Some(txn) = self
                    .transactions
                    .get_mut(&txn_pointer.sender)
                    .and_then(|txns| txns.remove(&txn_pointer.sequence_number))
                {
                    debug!(
                        LogSchema::new(LogEntry::MempoolFullEvictedTxn).txns(TxnsLog::new_txn(
                            txn.get_sender(),
                            txn.sequence_info.transaction_sequence_number
                        ))
                    );
                    evicted_bytes += txn.get_estimated_bytes() as u64;
                    evicted_txns += 1;
                    self.index_remove(&txn);
                    if !self.is_full() {
                        break;
                    }
                } else {
                    error!("Transaction not found in mempool while evicting from parking lot");
                    break;
                }
            }
            if evicted_txns > 0 {
                counters::CORE_MEMPOOL_PARKING_LOT_EVICTED_COUNT.observe(evicted_txns as f64);
                counters::CORE_MEMPOOL_PARKING_LOT_EVICTED_BYTES.observe(evicted_bytes as f64);
                counters::CORE_MEMPOOL_PARKING_LOT_EVICTED_LATENCY
                    .observe(now.elapsed().as_secs_f64());
            }
        }
        self.is_full()
    }

    fn is_full(&self) -> bool {
        self.system_ttl_index.size() >= self.capacity || self.size_bytes >= self.capacity_bytes
    }

    /// Check if a transaction would be ready for broadcast in mempool upon insertion (without inserting it).
    /// Two ways this can happen:
    /// 1. txn sequence number == curr_sequence_number
    ///    (this handles both cases where, (1) txn is first possible txn for an account and (2) the
    ///    previous txn is committed).
    /// 2. The txn before this is ready for broadcast but not yet committed.
    fn check_txn_ready(&self, txn: &MempoolTransaction, curr_sequence_number: u64) -> bool {
        let tx_sequence_number = txn.sequence_info.transaction_sequence_number;
        if tx_sequence_number == curr_sequence_number {
            return true;
        } else if tx_sequence_number == 0 {
            // shouldn't really get here because filtering out old txn sequence numbers happens earlier in workflow
            unreachable!("[mempool] already committed txn detected, cannot be checked for readiness upon insertion");
        }

        // check previous txn in sequence is ready
        if let Some(account_txns) = self.transactions.get(&txn.get_sender()) {
            if let Some(prev_txn) = account_txns.get(&(tx_sequence_number - 1)) {
                if let TimelineState::Ready(_) = prev_txn.timeline_state {
                    return true;
                }
            }
        }
        false
    }

    fn log_ready_transaction(
        ranking_score: u64,
        bucket: &str,
        insertion_info: &mut InsertionInfo,
        broadcast_ready: bool,
        priority: &str,
    ) {
        insertion_info.ready_time = SystemTime::now();
        if let Ok(time_delta) = SystemTime::now().duration_since(insertion_info.insertion_time) {
            let submitted_by = insertion_info.submitted_by_label();
            if broadcast_ready {
                counters::core_mempool_txn_commit_latency(
                    CONSENSUS_READY_LABEL,
                    submitted_by,
                    bucket,
                    time_delta,
                    priority,
                );
                counters::core_mempool_txn_commit_latency(
                    BROADCAST_READY_LABEL,
                    submitted_by,
                    bucket,
                    time_delta,
                    priority,
                );
            } else {
                counters::core_mempool_txn_commit_latency(
                    CONSENSUS_READY_LABEL,
                    submitted_by,
                    bucket,
                    time_delta,
                    priority,
                );
            }
        }

        if broadcast_ready {
            counters::core_mempool_txn_ranking_score(
                BROADCAST_READY_LABEL,
                BROADCAST_READY_LABEL,
                bucket,
                ranking_score,
            );
        }
        counters::core_mempool_txn_ranking_score(
            CONSENSUS_READY_LABEL,
            CONSENSUS_READY_LABEL,
            bucket,
            ranking_score,
        );
    }

    /// Maintains the following invariants:
    /// - All transactions of a given account that are sequential to the current sequence number
    ///   should be included in both the PriorityIndex (ordering for Consensus) and
    ///   TimelineIndex (txns for SharedMempool).
    /// - Other txns are considered to be "non-ready" and should be added to ParkingLotIndex.
    fn process_ready_transactions(&mut self, address: &AccountAddress, sequence_num: u64) {
        let sender_bucket = sender_bucket(address, self.num_sender_buckets);
        if let Some(txns) = self.transactions.get_mut(address) {
            let mut min_seq = sequence_num;

            while let Some(txn) = txns.get_mut(&min_seq) {
                let process_ready = !self.priority_index.contains(txn);

                self.priority_index.insert(txn);

                let process_broadcast_ready = txn.timeline_state == TimelineState::NotReady;
                if process_broadcast_ready {
                    self.timeline_index
                        .get_mut(&sender_bucket)
                        .unwrap()
                        .insert(txn);
                }

                if process_ready {
                    let bucket = self
                        .timeline_index
                        .get(&sender_bucket)
                        .unwrap()
                        .get_bucket(txn.ranking_score)
                        .to_string();
                    let bucket = format!("{}_{}", sender_bucket, bucket);

                    Self::log_ready_transaction(
                        txn.ranking_score,
                        bucket.as_str(),
                        &mut txn.insertion_info,
                        process_broadcast_ready,
                        txn.priority_of_sender
                            .clone()
                            .map_or_else(|| "Unknown".to_string(), |priority| priority.to_string())
                            .as_str(),
                    );
                }

                // Remove txn from parking lot after it has been promoted to
                // priority_index / timeline_index, i.e., txn status is ready.
                self.parking_lot_index.remove(txn);
                min_seq += 1;
            }

            let mut parking_lot_txns = 0;
            for (_, txn) in txns.range_mut((Bound::Excluded(min_seq), Bound::Unbounded)) {
                match txn.timeline_state {
                    TimelineState::Ready(_) => {},
                    _ => {
                        self.parking_lot_index.insert(txn);
                        parking_lot_txns += 1;
                    },
                }
            }

            trace!(
                LogSchema::new(LogEntry::ProcessReadyTxns).account(*address),
                first_ready_seq_num = sequence_num,
                last_ready_seq_num = min_seq,
                num_parked_txns = parking_lot_txns,
            );
            self.track_indices();
        }
    }

    fn clean_committed_transactions(&mut self, address: &AccountAddress, sequence_number: u64) {
        // Remove all previous seq number transactions for this account.
        // This can happen if transactions are sent to multiple nodes and one of the
        // nodes has sent the transaction to consensus but this node still has the
        // transaction sitting in mempool.
        if let Some(txns) = self.transactions.get_mut(address) {
            let mut active = txns.split_off(&sequence_number);
            let txns_for_removal = txns.clone();
            txns.clear();
            txns.append(&mut active);

            let mut rm_txns = match aptos_logger::enabled!(Level::Trace) {
                true => TxnsLog::new(),
                false => TxnsLog::new_with_max(10),
            };
            for transaction in txns_for_removal.values() {
                rm_txns.add(
                    transaction.get_sender(),
                    transaction.sequence_info.transaction_sequence_number,
                );
                self.index_remove(transaction);
            }
            trace!(
                LogSchema::new(LogEntry::CleanCommittedTxn).txns(rm_txns),
                "txns cleaned with committing tx {}:{}",
                address,
                sequence_number
            );
        }
    }

    /// Handles transaction commit.
    /// It includes deletion of all transactions with sequence number <= `account_sequence_number`
    /// and potential promotion of sequential txns to PriorityIndex/TimelineIndex.
    pub fn commit_transaction(&mut self, account: &AccountAddress, sequence_number: u64) {
        let current_seq_number = self.get_sequence_number(account).map_or(0, |v| *v);
        let new_seq_number = max(current_seq_number, sequence_number + 1);
        self.sequence_numbers.insert(*account, new_seq_number);
        self.clean_committed_transactions(account, new_seq_number);
        self.process_ready_transactions(account, new_seq_number);
    }

    pub fn reject_transaction(
        &mut self,
        account: &AccountAddress,
        sequence_number: u64,
        hash: &HashValue,
    ) {
        let mut txn_to_remove = None;
        if let Some((indexed_account, indexed_sequence_number)) = self.hash_index.get(hash) {
            if account == indexed_account && sequence_number == *indexed_sequence_number {
                txn_to_remove = self.get_mempool_txn(account, sequence_number).cloned();
            }
        }
        if let Some(txn_to_remove) = txn_to_remove {
            if let Some(txns) = self.transactions.get_mut(account) {
                txns.remove(&sequence_number);
            }
            self.index_remove(&txn_to_remove);

            if aptos_logger::enabled!(Level::Trace) {
                let mut txns_log = TxnsLog::new();
                txns_log.add(
                    txn_to_remove.get_sender(),
                    txn_to_remove.sequence_info.transaction_sequence_number,
                );
                trace!(LogSchema::new(LogEntry::CleanRejectedTxn).txns(txns_log));
            }
        }
    }

    /// Removes transaction from all indexes. Only call after removing from main transactions DS.
    fn index_remove(&mut self, txn: &MempoolTransaction) {
        counters::CORE_MEMPOOL_REMOVED_TXNS.inc();
        self.system_ttl_index.remove(txn);
        self.expiration_time_index.remove(txn);
        self.priority_index.remove(txn);
        let sender_bucket = sender_bucket(&txn.get_sender(), self.num_sender_buckets);
        self.timeline_index
            .get_mut(&sender_bucket)
            .unwrap_or_else(|| {
                panic!(
                    "Unable to get the timeline index for the sender bucket {}",
                    sender_bucket
                )
            })
            .remove(txn);
        self.parking_lot_index.remove(txn);
        self.hash_index.remove(&txn.get_committed_hash());
        self.size_bytes -= txn.get_estimated_bytes();

        // Remove account datastructures if there are no more transactions for the account.
        let address = &txn.get_sender();
        if let Some(txns) = self.transactions.get(address) {
            if txns.is_empty() {
                self.transactions.remove(address);
                self.sequence_numbers.remove(address);
            }
        }

        self.track_indices();
    }

    /// Read at most `count` transactions from timeline since `timeline_id`.
    /// This method takes into account the max number of bytes per transaction batch.
    /// Returns block of transactions along with their transaction ready times
    /// and new last_timeline_id.
    pub(crate) fn read_timeline(
        &self,
        sender_bucket: MempoolSenderBucket,
        timeline_id: &MultiBucketTimelineIndexIds,
        count: usize,
        before: Option<Instant>,
        // The priority of the receipient of the transactions
        priority_of_receiver: BroadcastPeerPriority,
    ) -> (Vec<(SignedTransaction, u64)>, MultiBucketTimelineIndexIds) {
        let mut batch = vec![];
        let mut batch_total_bytes: u64 = 0;
        let mut last_timeline_id = timeline_id.id_per_bucket.clone();

        // Add as many transactions to the batch as possible
        for (i, bucket) in self
            .timeline_index
            .get(&sender_bucket)
            .unwrap_or_else(|| {
                panic!(
                    "Unable to get the timeline index for the sender bucket {}",
                    sender_bucket
                )
            })
            .read_timeline(timeline_id, count, before)
            .iter()
            .enumerate()
            .rev()
        {
            for (address, sequence_number) in bucket {
                if let Some(txn) = self.get_mempool_txn(address, *sequence_number) {
                    let transaction_bytes = txn.txn.raw_txn_bytes_len() as u64;
                    if batch_total_bytes.saturating_add(transaction_bytes) > self.max_batch_bytes {
                        break; // The batch is full
                    } else {
                        batch.push((
                            txn.txn.clone(),
                            aptos_infallible::duration_since_epoch_at(
                                &txn.insertion_info.ready_time,
                            )
                            .as_millis() as u64,
                        ));
                        batch_total_bytes = batch_total_bytes.saturating_add(transaction_bytes);
                        if let TimelineState::Ready(timeline_id) = txn.timeline_state {
                            last_timeline_id[i] = timeline_id;
                        }
                        let bucket = self.get_bucket(txn.ranking_score, &txn.get_sender());
                        Mempool::log_txn_latency(
                            &txn.insertion_info,
                            bucket.as_str(),
                            BROADCAST_BATCHED_LABEL,
                            priority_of_receiver.to_string().as_str(),
                        );
                        counters::core_mempool_txn_ranking_score(
                            BROADCAST_BATCHED_LABEL,
                            BROADCAST_BATCHED_LABEL,
                            bucket.as_str(),
                            txn.ranking_score,
                        );
                    }
                }
            }
        }

        (batch, last_timeline_id.into())
    }

    pub(crate) fn timeline_range(
        &self,
        sender_bucket: MempoolSenderBucket,
        start_end_pairs: HashMap<TimelineIndexIdentifier, (u64, u64)>,
    ) -> Vec<(SignedTransaction, u64)> {
        self.timeline_index
            .get(&sender_bucket)
            .unwrap_or_else(|| {
                panic!(
                    "Unable to get the timeline index for the sender bucket {}",
                    sender_bucket
                )
            })
            .timeline_range(start_end_pairs)
            .iter()
            .filter_map(|(account, sequence_number)| {
                self.transactions
                    .get(account)
                    .and_then(|txns| txns.get(sequence_number))
                    .map(|txn| {
                        (
                            txn.txn.clone(),
                            aptos_infallible::duration_since_epoch_at(
                                &txn.insertion_info.ready_time,
                            )
                            .as_millis() as u64,
                        )
                    })
            })
            .collect()
    }

    /// If the oldest transaction (that never entered parking lot) is larger than
    /// eager_expire_threshold, there is significant backlog so add eager_expire_time
    fn eager_expire_time(&self, gc_time: Duration) -> Duration {
        let eager_expire_threshold = match self.eager_expire_threshold {
            None => {
                return gc_time;
            },
            Some(v) => v,
        };

        let mut oldest_insertion_time = None;
        // Limit the worst-case linear search to 20.
        for key in self.system_ttl_index.iter().take(20) {
            if let Some(txn) = self.get_mempool_txn(&key.address, key.sequence_number) {
                if !txn.was_parked {
                    oldest_insertion_time = Some(txn.insertion_info.insertion_time);
                    break;
                }
            }
        }
        if let Some(insertion_time) = oldest_insertion_time {
            if let Ok(age) = SystemTime::now().duration_since(insertion_time) {
                if age > eager_expire_threshold {
                    counters::CORE_MEMPOOL_GC_EAGER_EXPIRE_EVENT_COUNT.inc();
                    return gc_time.saturating_add(self.eager_expire_time);
                }
            }
        }
        gc_time
    }

    /// Garbage collect old transactions.
    pub(crate) fn gc_by_system_ttl(&mut self, gc_time: Duration) {
        self.gc(gc_time, true);
    }

    /// Garbage collect old transactions based on client-specified expiration time.
    pub(crate) fn gc_by_expiration_time(&mut self, block_time: Duration) {
        self.gc(self.eager_expire_time(block_time), false);
    }

    fn gc(&mut self, now: Duration, by_system_ttl: bool) {
        let (metric_label, index, log_event) = if by_system_ttl {
            (
                counters::GC_SYSTEM_TTL_LABEL,
                &mut self.system_ttl_index,
                LogEvent::SystemTTLExpiration,
            )
        } else {
            (
                counters::GC_CLIENT_EXP_LABEL,
                &mut self.expiration_time_index,
                LogEvent::ClientExpiration,
            )
        };
        counters::CORE_MEMPOOL_GC_EVENT_COUNT
            .with_label_values(&[metric_label])
            .inc();

        let mut gc_txns = index.gc(now);
        // sort the expired txns by order of sequence number per account
        gc_txns.sort_by_key(|key| (key.address, key.sequence_number));
        let mut gc_iter = gc_txns.iter().peekable();

        let mut gc_txns_log = match aptos_logger::enabled!(Level::Trace) {
            true => TxnsLog::new(),
            false => TxnsLog::new_with_max(10),
        };
        while let Some(key) = gc_iter.next() {
            if let Some(txns) = self.transactions.get_mut(&key.address) {
                let park_range_start = Bound::Excluded(key.sequence_number);
                let park_range_end = gc_iter
                    .peek()
                    .filter(|next_key| key.address == next_key.address)
                    .map_or(Bound::Unbounded, |next_key| {
                        Bound::Excluded(next_key.sequence_number)
                    });
                // mark all following txns as non-ready, i.e. park them
                for (_, t) in txns.range_mut((park_range_start, park_range_end)) {
                    self.parking_lot_index.insert(t);
                    self.priority_index.remove(t);
                    let sender_bucket = sender_bucket(&t.get_sender(), self.num_sender_buckets);
                    self.timeline_index
                        .get_mut(&sender_bucket)
                        .unwrap_or_else(|| {
                            panic!(
                                "Unable to get the timeline index for the sender bucket {}",
                                sender_bucket
                            )
                        })
                        .remove(t);
                    if let TimelineState::Ready(_) = t.timeline_state {
                        t.timeline_state = TimelineState::NotReady;
                    }
                }
                if let Some(txn) = txns.remove(&key.sequence_number) {
                    let is_active = self.priority_index.contains(&txn);
                    let status = if is_active {
                        counters::GC_ACTIVE_TXN_LABEL
                    } else {
                        counters::GC_PARKED_TXN_LABEL
                    };
                    let account = txn.get_sender();
                    let txn_sequence_number = txn.sequence_info.transaction_sequence_number;
                    gc_txns_log.add_with_status(account, txn_sequence_number, status);
                    if let Ok(time_delta) =
                        SystemTime::now().duration_since(txn.insertion_info.insertion_time)
                    {
                        counters::CORE_MEMPOOL_GC_LATENCY
                            .with_label_values(&[metric_label, status])
                            .observe(time_delta.as_secs_f64());
                    }

                    // remove txn
                    self.index_remove(&txn);
                }
            }
        }

        if !gc_txns_log.is_empty() {
            debug!(LogSchema::event_log(LogEntry::GCRemoveTxns, log_event).txns(gc_txns_log));
        } else {
            trace!(LogSchema::event_log(LogEntry::GCRemoveTxns, log_event).txns(gc_txns_log));
        }
        self.track_indices();
    }

    pub(crate) fn iter_queue(&self) -> PriorityQueueIter {
        self.priority_index.iter()
    }

    pub(crate) fn gen_snapshot(&self) -> TxnsLog {
        let mut txns_log = TxnsLog::new();
        for (account, txns) in self.transactions.iter() {
            for (seq_num, txn) in txns.iter() {
                let status =
                    if self
                        .parking_lot_index
                        .contains(account, *seq_num, txn.get_committed_hash())
                    {
                        "parked"
                    } else {
                        "ready"
                    };
                txns_log.add_full_metadata(
                    *account,
                    *seq_num,
                    status,
                    txn.insertion_info.insertion_time,
                );
            }
        }
        txns_log
    }

    #[cfg(test)]
    pub(crate) fn get_parking_lot_size(&self) -> usize {
        self.parking_lot_index.size()
    }

    #[cfg(test)]
    pub(crate) fn get_transactions(&self) -> &HashMap<AccountAddress, AccountTransactions> {
        &self.transactions
    }

    pub(crate) fn get_parking_lot_addresses(&self) -> Vec<(AccountAddress, u64)> {
        self.parking_lot_index.get_addresses()
    }
}
