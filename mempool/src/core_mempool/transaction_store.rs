// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{
        index::{
            AccountTransactions, ParkingLotIndex, PriorityIndex, PriorityQueueIter, TTLIndex,
            TimelineIndex,
        },
        transaction::{MempoolTransaction, TimelineState},
    },
    counters,
    logging::{LogEntry, LogEvent, LogSchema, TxnsLog},
};
use aptos_config::config::MempoolConfig;
use aptos_crypto::HashValue;
use aptos_logger::{prelude::*, Level};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountSequenceInfo,
    mempool_status::{MempoolStatus, MempoolStatusCode},
    transaction::SignedTransaction,
};
use std::cmp::max;
use std::mem::size_of;
use std::{
    collections::HashMap,
    ops::Bound,
    time::{Duration, SystemTime},
};

/// Estimated per-txn overhead of indexes. Needs to be updated if additional indexes are added.
pub const TXN_INDEX_ESTIMATED_BYTES: usize = size_of::<crate::core_mempool::index::OrderedQueueKey>() // priority_index
    + size_of::<crate::core_mempool::index::TTLOrderingKey>() * 2 // expiration_time_index + system_ttl_index
    + (size_of::<u64>() * 3 + size_of::<AccountAddress>()) // timeline_index
    + (size_of::<HashValue>() + size_of::<u64>() + size_of::<AccountAddress>()); // hash_index

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
    timeline_index: TimelineIndex,
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
}

impl TransactionStore {
    pub(crate) fn new(config: &MempoolConfig) -> Self {
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
            timeline_index: TimelineIndex::new(),
            parking_lot_index: ParkingLotIndex::new(),
            hash_index: HashMap::new(),

            // estimated size in bytes
            size_bytes: 0,

            // configuration
            capacity: config.capacity,
            capacity_bytes: config.capacity_bytes,
            capacity_per_user: config.capacity_per_user,
            max_batch_bytes: config.shared_mempool_max_batch_bytes,
        }
    }

    /// Fetch transaction by account address + sequence_number.
    pub(crate) fn get(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<SignedTransaction> {
        if let Some(txn) = self
            .transactions
            .get(address)
            .and_then(|txns| txns.get(&sequence_number))
        {
            return Some(txn.txn.clone());
        }
        None
    }

    pub(crate) fn get_by_hash(&self, hash: HashValue) -> Option<SignedTransaction> {
        match self.hash_index.get(&hash) {
            Some((address, seq)) => self.get(address, *seq),
            None => None,
        }
    }

    pub(crate) fn get_insertion_time(
        &self,
        address: &AccountAddress,
        sequence_number: u64,
    ) -> Option<&SystemTime> {
        if let Some(txn) = self
            .transactions
            .get(address)
            .and_then(|txns| txns.get(&sequence_number))
        {
            if txn.timeline_state != TimelineState::NonQualified {
                return Some(&txn.insertion_time);
            }
        }
        None
    }

    pub(crate) fn remove(
        &mut self,
        sender: &AccountAddress,
        sequence_number: u64,
        is_rejected: bool,
    ) {
        let current_seq_number = self.get_sequence_number(sender).map_or(0, |v| *v);
        if is_rejected {
            if sequence_number >= current_seq_number {
                self.reject_transaction(sender, sequence_number);
            }
        } else {
            let new_seq_number =
                AccountSequenceInfo::Sequential(max(current_seq_number, sequence_number + 1));
            self.sequence_numbers
                .insert(*sender, new_seq_number.min_seq());
            self.commit_transaction(sender, new_seq_number);
        }
    }

    pub(crate) fn get_sequence_number(&self, address: &AccountAddress) -> Option<&u64> {
        self.sequence_numbers.get(address)
    }

    /// Insert transaction into TransactionStore. Performs validation checks and updates indexes.
    pub(crate) fn insert(&mut self, txn: MempoolTransaction) -> MempoolStatus {
        let address = txn.get_sender();
        let sequence_number = txn.sequence_info;

        // If the transaction is already in Mempool, we only allow the user to
        // increase the gas unit price to speed up a transaction, but not the max gas.
        //
        // Transactions with all the same inputs (but possibly signed differently) are idempotent
        // since the raw transaction is the same
        if let Some(txns) = self.transactions.get_mut(&address) {
            if let Some(current_version) =
                txns.get_mut(&sequence_number.transaction_sequence_number)
            {
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
                } else if current_version.txn.gas_unit_price() < txn.get_gas_price() {
                    // Update txn if gas unit price is a larger value than before
                    if let Some(txn) = txns.remove(&sequence_number.transaction_sequence_number) {
                        self.index_remove(&txn);
                    };
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

        if self.check_is_full_after_eviction(
            &txn,
            sequence_number.account_sequence_number_type.min_seq(),
        ) {
            return MempoolStatus::new(MempoolStatusCode::MempoolIsFull).with_message(format!(
                "Mempool is full. Mempool size: {}, Capacity: {}",
                self.system_ttl_index.size(),
                self.capacity,
            ));
        }

        self.clean_committed_transactions(
            &address,
            sequence_number.account_sequence_number_type.min_seq(),
        );

        self.transactions
            .entry(address)
            .or_insert_with(AccountTransactions::new);

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
            let sender = txn.get_sender();
            self.system_ttl_index.insert(&txn);
            self.expiration_time_index.insert(&txn);
            self.hash_index.insert(
                txn.get_committed_hash(),
                (sender, sequence_number.transaction_sequence_number),
            );
            let txn_size_bytes = txn.get_estimated_bytes();
            txns.insert(sequence_number.transaction_sequence_number, txn);
            self.sequence_numbers.insert(
                sender,
                sequence_number.account_sequence_number_type.min_seq(),
            );
            self.size_bytes += txn_size_bytes;
            self.track_indices();
        }
        self.process_ready_transactions(&address, sequence_number.account_sequence_number_type);
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
            self.timeline_index.size(),
        );
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
            // try to free some space in Mempool from ParkingLot by evicting a non-ready txn
            if let Some((address, sequence_number)) = self.parking_lot_index.get_poppable() {
                if let Some(txn) = self
                    .transactions
                    .get_mut(&address)
                    .and_then(|txns| txns.remove(&sequence_number))
                {
                    debug!(
                        LogSchema::new(LogEntry::MempoolFullEvictedTxn).txns(TxnsLog::new_txn(
                            txn.get_sender(),
                            txn.sequence_info.transaction_sequence_number
                        ))
                    );
                    self.index_remove(&txn);
                }
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
    /// (this handles both cases where, (1) txn is first possible txn for an account and (2) the
    /// previous txn is committed).
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

    /// Maintains the following invariants:
    /// - All transactions of a given account that are sequential to the current sequence number
    ///   should be included in both the PriorityIndex (ordering for Consensus) and
    ///   TimelineIndex (txns for SharedMempool).
    /// - Other txns are considered to be "non-ready" and should be added to ParkingLotIndex.
    fn process_ready_transactions(
        &mut self,
        address: &AccountAddress,
        sequence_info: AccountSequenceInfo,
    ) {
        if let Some(txns) = self.transactions.get_mut(address) {
            let mut min_seq = sequence_info.min_seq();

            match sequence_info {
                AccountSequenceInfo::Sequential(_) => {
                    while let Some(txn) = txns.get_mut(&min_seq) {
                        self.priority_index.insert(txn);

                        if txn.timeline_state == TimelineState::NotReady {
                            self.timeline_index.insert(txn);
                        }

                        // Remove txn from parking lot after it has been promoted to
                        // priority_index / timeline_index, i.e., txn status is ready.
                        self.parking_lot_index.remove(txn);
                        min_seq += 1;
                    }
                }
            }

            let mut parking_lot_txns = 0;
            for (_, txn) in txns.range_mut((Bound::Excluded(min_seq), Bound::Unbounded)) {
                match txn.timeline_state {
                    TimelineState::Ready(_) => {}
                    _ => {
                        self.parking_lot_index.insert(txn);
                        parking_lot_txns += 1;
                    }
                }
            }
            trace!(
                LogSchema::new(LogEntry::ProcessReadyTxns).account(*address),
                first_ready_seq_num = sequence_info.min_seq(),
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
    fn commit_transaction(
        &mut self,
        account: &AccountAddress,
        account_sequence_number: AccountSequenceInfo,
    ) {
        self.clean_committed_transactions(account, account_sequence_number.min_seq());
        self.process_ready_transactions(account, account_sequence_number);
    }

    fn reject_transaction(&mut self, account: &AccountAddress, _sequence_number: u64) {
        if let Some(txns) = self.transactions.remove(account) {
            let mut txns_log = match aptos_logger::enabled!(Level::Trace) {
                true => TxnsLog::new(),
                false => TxnsLog::new_with_max(10),
            };
            for transaction in txns.values() {
                txns_log.add(
                    transaction.get_sender(),
                    transaction.sequence_info.transaction_sequence_number,
                );
                self.index_remove(transaction);
            }
            debug!(LogSchema::new(LogEntry::CleanRejectedTxn).txns(txns_log));
        }
    }

    /// Removes transaction from all indexes. Only call after removing from main transactions DS.
    fn index_remove(&mut self, txn: &MempoolTransaction) {
        counters::CORE_MEMPOOL_REMOVED_TXNS.inc();
        self.system_ttl_index.remove(txn);
        self.expiration_time_index.remove(txn);
        self.priority_index.remove(txn);
        self.timeline_index.remove(txn);
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
    /// Returns block of transactions and new last_timeline_id.
    pub(crate) fn read_timeline(
        &self,
        timeline_id: u64,
        count: usize,
    ) -> (Vec<SignedTransaction>, u64) {
        let mut batch = vec![];
        let mut batch_total_bytes: u64 = 0;
        let mut last_timeline_id = timeline_id;

        // Add as many transactions to the batch as possible
        for (address, sequence_number) in self.timeline_index.read_timeline(timeline_id, count) {
            if let Some(txn) = self
                .transactions
                .get(&address)
                .and_then(|txns| txns.get(&sequence_number))
            {
                let transaction_bytes = txn.txn.raw_txn_bytes_len() as u64;
                if batch_total_bytes.saturating_add(transaction_bytes) > self.max_batch_bytes {
                    break; // The batch is full
                } else {
                    batch.push(txn.txn.clone());
                    batch_total_bytes = batch_total_bytes.saturating_add(transaction_bytes);
                    if let TimelineState::Ready(timeline_id) = txn.timeline_state {
                        last_timeline_id = timeline_id;
                    }
                }
            }
        }

        (batch, last_timeline_id)
    }

    pub(crate) fn timeline_range(&self, start_id: u64, end_id: u64) -> Vec<SignedTransaction> {
        self.timeline_index
            .timeline_range(start_id, end_id)
            .iter()
            .filter_map(|(account, sequence_number)| {
                self.transactions
                    .get(account)
                    .and_then(|txns| txns.get(sequence_number))
                    .map(|txn| txn.txn.clone())
            })
            .collect()
    }

    /// Garbage collect old transactions.
    pub(crate) fn gc_by_system_ttl(&mut self, gc_time: Duration) {
        self.gc(gc_time, true);
    }

    /// Garbage collect old transactions based on client-specified expiration time.
    pub(crate) fn gc_by_expiration_time(&mut self, block_time: Duration) {
        self.gc(block_time, false);
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
                    self.timeline_index.remove(t);
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
                    if let Ok(time_delta) = SystemTime::now().duration_since(txn.insertion_time) {
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
                let status = if self.parking_lot_index.contains(account, seq_num) {
                    "parked"
                } else {
                    "ready"
                };
                txns_log.add_full_metadata(*account, *seq_num, status, txn.insertion_time);
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
}
