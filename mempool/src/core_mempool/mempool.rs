// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Mempool is used to track transactions which have been submitted but not yet
//! agreed upon.
use crate::{
    core_mempool::{
        index::TxnPointer,
        transaction::{InsertionInfo, MempoolTransaction, TimelineState},
        transaction_store::{sender_bucket, TransactionStore},
    },
    counters,
    logging::{LogEntry, LogSchema, TxnsLog},
    network::BroadcastPeerPriority,
    shared_mempool::types::{
        MempoolSenderBucket, MultiBucketTimelineIndexIds, TimelineIndexIdentifier,
    },
};
use aptos_config::config::NodeConfig;
use aptos_consensus_types::common::{TransactionInProgress, TransactionSummary};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::{MempoolStatus, MempoolStatusCode},
    transaction::{use_case::UseCaseKey, ReplayProtector, SignedTransaction},
    vm_status::DiscardedVMStatus,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::atomic::Ordering,
    time::{Duration, Instant, SystemTime},
};

pub struct Mempool {
    // Stores the metadata of all transactions in mempool (of all states).
    pub(crate) transactions: TransactionStore,

    pub system_transaction_timeout: Duration,
}

impl Mempool {
    pub fn new(config: &NodeConfig) -> Self {
        Mempool {
            transactions: TransactionStore::new(&config.mempool),
            system_transaction_timeout: Duration::from_secs(
                config.mempool.system_transaction_timeout_secs,
            ),
        }
    }

    /// This function will be called once the transaction has been stored.
    pub(crate) fn commit_transaction(
        &mut self,
        sender: &AccountAddress,
        replay_protector: ReplayProtector,
    ) {
        self.transactions
            .commit_transaction(sender, replay_protector);
    }

    pub(crate) fn log_commit_transaction(
        &self,
        sender: &AccountAddress,
        replay_protector: ReplayProtector,
        tracked_use_case: Option<(UseCaseKey, &String)>,
        block_timestamp: Duration,
    ) {
        trace!(
            LogSchema::new(LogEntry::RemoveTxn).txns(TxnsLog::new_txn(*sender, replay_protector)),
            is_rejected = false
        );
        self.log_commit_latency(*sender, replay_protector, tracked_use_case, block_timestamp);
        if let Some(ranking_score) = self
            .transactions
            .get_ranking_score(sender, replay_protector)
        {
            counters::core_mempool_txn_ranking_score(
                counters::REMOVE_LABEL,
                counters::COMMIT_ACCEPTED_LABEL,
                self.transactions.get_bucket(ranking_score, sender).as_str(),
                ranking_score,
            );
        }
    }

    fn log_reject_transaction(
        &self,
        sender: &AccountAddress,
        replay_protector: ReplayProtector,
        reason_label: &'static str,
    ) {
        trace!(
            LogSchema::new(LogEntry::RemoveTxn).txns(TxnsLog::new_txn(*sender, replay_protector)),
            is_rejected = true,
            label = reason_label,
        );
        self.log_commit_rejected_latency(*sender, replay_protector, reason_label);
        if let Some(ranking_score) = self
            .transactions
            .get_ranking_score(sender, replay_protector)
        {
            counters::core_mempool_txn_ranking_score(
                counters::REMOVE_LABEL,
                reason_label,
                self.transactions.get_bucket(ranking_score, sender).as_str(),
                ranking_score,
            );
        }
    }

    pub(crate) fn reject_transaction(
        &mut self,
        sender: &AccountAddress,
        replay_protector: ReplayProtector,
        hash: &HashValue,
        reason: &DiscardedVMStatus,
    ) {
        if *reason == DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW {
            self.log_reject_transaction(sender, replay_protector, counters::COMMIT_IGNORED_LABEL);
            // Do not remove the transaction from mempool
            return;
        }

        let label = if *reason == DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD {
            counters::COMMIT_REJECTED_DUPLICATE_LABEL
        } else {
            counters::COMMIT_REJECTED_LABEL
        };
        self.log_reject_transaction(sender, replay_protector, label);
        self.transactions
            .reject_transaction(sender, replay_protector, hash);
    }

    pub(crate) fn log_txn_latency(
        insertion_info: &InsertionInfo,
        bucket: &str,
        stage: &'static str,
        priority: &str,
    ) {
        if let Ok(time_delta) = SystemTime::now().duration_since(insertion_info.insertion_time) {
            counters::core_mempool_txn_commit_latency(
                stage,
                insertion_info.submitted_by_label(),
                bucket,
                time_delta,
                priority,
            );
        }
    }

    fn log_consensus_pulled_latency(
        &self,
        account: AccountAddress,
        replay_protector: ReplayProtector,
    ) {
        if let Some((insertion_info, bucket, priority)) = self
            .transactions
            .get_insertion_info_and_bucket(&account, replay_protector)
        {
            let prev_count = insertion_info
                .consensus_pulled_counter
                .fetch_add(1, Ordering::Relaxed);
            Self::log_txn_latency(
                insertion_info,
                bucket.as_str(),
                counters::CONSENSUS_PULLED_LABEL,
                priority.as_str(),
            );
            counters::CORE_MEMPOOL_TXN_CONSENSUS_PULLED_BY_BUCKET
                .with_label_values(&[bucket.as_str()])
                .observe((prev_count + 1) as f64);
        }
    }

    fn log_commit_rejected_latency(
        &self,
        account: AccountAddress,
        replay_protector: ReplayProtector,
        stage: &'static str,
    ) {
        if let Some((insertion_info, bucket, priority)) = self
            .transactions
            .get_insertion_info_and_bucket(&account, replay_protector)
        {
            Self::log_txn_latency(insertion_info, bucket.as_str(), stage, priority.as_str());
        }
    }

    fn log_commit_and_parked_latency(
        insertion_info: &InsertionInfo,
        bucket: &str,
        priority: &str,
        tracked_use_case: Option<(UseCaseKey, &String)>,
    ) {
        let parked_duration = if let Some(park_time) = insertion_info.park_time {
            let parked_duration = insertion_info
                .ready_time
                .duration_since(park_time)
                .unwrap_or(Duration::ZERO);
            counters::core_mempool_txn_commit_latency(
                counters::PARKED_TIME_LABEL,
                insertion_info.submitted_by_label(),
                bucket,
                parked_duration,
                priority,
            );
            parked_duration
        } else {
            Duration::ZERO
        };

        if let Ok(commit_duration) = SystemTime::now().duration_since(insertion_info.insertion_time)
        {
            let commit_minus_parked = commit_duration
                .checked_sub(parked_duration)
                .unwrap_or(Duration::ZERO);
            counters::core_mempool_txn_commit_latency(
                counters::NON_PARKED_COMMIT_ACCEPTED_LABEL,
                insertion_info.submitted_by_label(),
                bucket,
                commit_minus_parked,
                priority,
            );

            if insertion_info.park_time.is_none() {
                let use_case_label = tracked_use_case
                    .as_ref()
                    .map_or("entry_user_other", |(_, use_case_name)| {
                        use_case_name.as_str()
                    });

                counters::TXN_E2E_USE_CASE_COMMIT_LATENCY
                    .with_label_values(&[
                        use_case_label,
                        insertion_info.submitted_by_label(),
                        bucket,
                    ])
                    .observe(commit_duration.as_secs_f64());
            }
        }
    }

    fn log_commit_latency(
        &self,
        account: AccountAddress,
        replay_protector: ReplayProtector,
        tracked_use_case: Option<(UseCaseKey, &String)>,
        block_timestamp: Duration,
    ) {
        if let Some((insertion_info, bucket, priority)) = self
            .transactions
            .get_insertion_info_and_bucket(&account, replay_protector)
        {
            Self::log_txn_latency(
                insertion_info,
                bucket.as_str(),
                counters::COMMIT_ACCEPTED_LABEL,
                priority.as_str(),
            );
            Self::log_commit_and_parked_latency(
                insertion_info,
                bucket.as_str(),
                priority.as_str(),
                tracked_use_case,
            );

            let insertion_timestamp =
                aptos_infallible::duration_since_epoch_at(&insertion_info.insertion_time);
            if let Some(insertion_to_block) = block_timestamp.checked_sub(insertion_timestamp) {
                counters::core_mempool_txn_commit_latency(
                    counters::COMMIT_ACCEPTED_BLOCK_LABEL,
                    insertion_info.submitted_by_label(),
                    bucket.as_str(),
                    insertion_to_block,
                    priority.to_string().as_str(),
                );
            }
        }
    }

    pub(crate) fn get_by_hash(&self, hash: HashValue) -> Option<SignedTransaction> {
        self.transactions.get_by_hash(hash)
    }

    /// Used to add a transaction to the Mempool.
    /// Performs basic validation: checks account's sequence number.
    pub(crate) fn add_txn(
        &mut self,
        txn: SignedTransaction,
        ranking_score: u64,
        // For orderless transactions, the sender's account_sequence_number is not fetched. account_sequence_number is None
        // For sequence number transactions, the sender's account_sequence_number is fetched. account_sequence_number is Some(u64)
        account_sequence_number: Option<u64>,
        timeline_state: TimelineState,
        client_submitted: bool,
        // The time at which the transaction was inserted into the mempool of the
        // downstream node (sender of the mempool transaction) in millis since epoch
        ready_time_at_sender: Option<u64>,
        // The prority of this node for the peer that sent the transaction
        priority: Option<BroadcastPeerPriority>,
    ) -> MempoolStatus {
        trace!(
            LogSchema::new(LogEntry::AddTxn)
                .txns(TxnsLog::new_txn(txn.sender(), txn.replay_protector())),
            committed_seq_number = account_sequence_number
        );

        if let ReplayProtector::SequenceNumber(txn_seq_num) = txn.replay_protector() {
            // don't accept old transactions (e.g. seq is less than account's current seq_number)
            match &account_sequence_number {
                Some(account_sequence_number) => {
                    if txn_seq_num < *account_sequence_number {
                        return MempoolStatus::new(MempoolStatusCode::InvalidSeqNumber)
                            .with_message(format!(
                                "transaction sequence number is {}, current sequence number is  {}",
                                txn_seq_num, account_sequence_number,
                            ));
                    }
                },
                None => {
                    return MempoolStatus::new(MempoolStatusCode::InvalidSeqNumber).with_message(
                        format!(
                            "transaction has sequence number {}, but not sequence number provided for sender's account",
                            txn_seq_num,
                        ),
                    );
                },
            }
        };

        let now = SystemTime::now();
        let expiration_time =
            aptos_infallible::duration_since_epoch_at(&now) + self.system_transaction_timeout;

        let sender = txn.sender();
        let txn_info = MempoolTransaction::new(
            txn.clone(),
            expiration_time,
            ranking_score,
            timeline_state,
            now,
            client_submitted,
            priority.clone(),
        );

        let submitted_by_label = txn_info.insertion_info.submitted_by_label();
        let status = self.transactions.insert(txn_info, account_sequence_number);
        let now = aptos_infallible::duration_since_epoch().as_millis() as u64;

        if status.code == MempoolStatusCode::Accepted {
            counters::SENDER_BUCKET_FREQUENCIES
                .with_label_values(&[sender_bucket(
                    &sender,
                    self.transactions.num_sender_buckets(),
                )
                .to_string()
                .as_str()])
                .inc();
            if let Some(ready_time_at_sender) = ready_time_at_sender {
                let bucket = self.transactions.get_bucket(ranking_score, &sender);
                counters::core_mempool_txn_commit_latency(
                    counters::BROADCAST_RECEIVED_LABEL,
                    submitted_by_label,
                    bucket.as_str(),
                    Duration::from_millis(now.saturating_sub(ready_time_at_sender)),
                    priority
                        .map_or_else(|| "Unknown".to_string(), |priority| priority.to_string())
                        .as_str(),
                );
            }
        }
        counters::core_mempool_txn_ranking_score(
            counters::INSERT_LABEL,
            status.code.to_string().as_str(),
            self.transactions
                .get_bucket(ranking_score, &sender)
                .as_str(),
            ranking_score,
        );
        status
    }

    /// Txn was already chosen, either in a local or remote previous pull (so now in consensus) or
    /// in the current pull.
    fn txn_was_chosen(
        account_address: AccountAddress,
        sequence_number: u64,
        inserted: &HashSet<(AccountAddress, ReplayProtector)>,
        exclude_transactions: &BTreeMap<TransactionSummary, TransactionInProgress>,
    ) -> bool {
        if inserted.contains(&(
            account_address,
            ReplayProtector::SequenceNumber(sequence_number),
        )) {
            return true;
        }

        // TODO: Make sure this range search works as expected
        let min_inclusive = TxnPointer::new(
            account_address,
            ReplayProtector::SequenceNumber(sequence_number),
            HashValue::zero(),
        );
        let max_exclusive = TxnPointer::new(
            account_address,
            ReplayProtector::SequenceNumber(sequence_number.saturating_add(1)),
            HashValue::zero(),
        );

        exclude_transactions
            .range(min_inclusive..max_exclusive)
            .next()
            .is_some()
    }

    /// Fetches next block of transactions for consensus.
    /// `return_non_full` - if false, only return transactions when max_txns or max_bytes is reached
    ///                     Should always be true for Quorum Store.
    /// `include_gas_upgraded` - Return transactions that had gas upgraded, even if they are in
    ///                          exclude_transactions. Should only be true for Quorum Store.
    /// `exclude_transactions` - transactions that were sent to Consensus but were not committed yet
    ///  mempool should filter out such transactions.
    #[allow(clippy::explicit_counter_loop)]
    pub(crate) fn get_batch(
        &self,
        max_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
        exclude_transactions: BTreeMap<TransactionSummary, TransactionInProgress>,
    ) -> Vec<SignedTransaction> {
        let start_time = Instant::now();
        let exclude_size = exclude_transactions.len();
        let mut inserted = HashSet::new();

        let gas_end_time = start_time.elapsed();

        let mut result = vec![];
        // Helper DS. Helps to mitigate scenarios where account submits several transactions
        // with increasing gas price (e.g. user submits transactions with sequence number 1, 2
        // and gas_price 1, 10 respectively)
        // Later txn has higher gas price and will be observed first in priority index iterator,
        // but can't be executed before first txn. Once observed, such txn will be saved in
        // `skipped` DS and rechecked once it's ancestor becomes available
        let mut skipped = HashSet::new();
        let mut total_bytes = 0;
        let mut txn_walked = 0usize;
        // iterate over the queue of transactions based on gas price
        'main: for txn in self.transactions.iter_queue() {
            txn_walked += 1;
            let txn_ptr = TxnPointer::from(txn);

            // TODO: removed gas upgraded logic. double check if it's needed
            if exclude_transactions.contains_key(&txn_ptr) {
                continue;
            }
            let txn_replay_protector = txn.replay_protector;
            match txn_replay_protector {
                ReplayProtector::SequenceNumber(txn_seq) => {
                    let txn_in_sequence = txn_seq > 0
                        && Self::txn_was_chosen(
                            txn.address,
                            txn_seq - 1,
                            &inserted,
                            &exclude_transactions,
                        );
                    let account_sequence_number =
                        self.transactions.get_account_sequence_number(&txn.address);
                    // include transaction if it's "next" for given account or
                    // we've already sent its ancestor to Consensus.
                    if txn_in_sequence || account_sequence_number == Some(&txn_seq) {
                        inserted.insert((txn.address, txn_replay_protector));
                        result.push((txn.address, txn_replay_protector));
                        if (result.len() as u64) == max_txns {
                            break;
                        }
                        // check if we can now include some transactions
                        // that were skipped before for given account
                        let (skipped_txn_sender, mut skipped_txn_seq_num) =
                            (txn.address, txn_seq + 1);
                        while skipped.remove(&(skipped_txn_sender, skipped_txn_seq_num)) {
                            inserted.insert((
                                skipped_txn_sender,
                                ReplayProtector::SequenceNumber(skipped_txn_seq_num),
                            ));
                            result.push((
                                skipped_txn_sender,
                                ReplayProtector::SequenceNumber(skipped_txn_seq_num),
                            ));
                            if (result.len() as u64) == max_txns {
                                break 'main;
                            }
                            skipped_txn_seq_num += 1;
                        }
                    } else {
                        skipped.insert((txn.address, txn_seq));
                    }
                },
                ReplayProtector::Nonce(_) => {
                    inserted.insert((txn.address, txn_replay_protector));
                    result.push((txn.address, txn_replay_protector));
                    if (result.len() as u64) == max_txns {
                        break;
                    }
                },
            };
        }
        let result_size = result.len();
        let result_end_time = start_time.elapsed();
        let result_time = result_end_time.saturating_sub(gas_end_time);

        let mut block = Vec::with_capacity(result_size);
        let mut full_bytes = false;
        for (sender, replay_protector) in result {
            if let Some((txn, ranking_score)) = self
                .transactions
                .get_with_ranking_score(&sender, replay_protector)
            {
                let txn_size = txn.txn_bytes_len() as u64;
                if total_bytes + txn_size > max_bytes {
                    full_bytes = true;
                    break;
                }
                total_bytes += txn_size;
                block.push(txn);
                if total_bytes == max_bytes {
                    full_bytes = true;
                }
                counters::core_mempool_txn_ranking_score(
                    counters::CONSENSUS_PULLED_LABEL,
                    counters::CONSENSUS_PULLED_LABEL,
                    self.transactions
                        .get_bucket(ranking_score, &sender)
                        .as_str(),
                    ranking_score,
                );
            }
        }
        let block_end_time = start_time.elapsed();
        let block_time = block_end_time.saturating_sub(result_end_time);

        if result_size > 0 {
            debug!(
                LogSchema::new(LogEntry::GetBlock),
                seen_consensus = exclude_size,
                walked = txn_walked,
                // before size and non full check
                result_size = result_size,
                // before non full check
                byte_size = total_bytes,
                block_size = block.len(),
                return_non_full = return_non_full,
                result_time_ms = result_time.as_millis(),
                block_time_ms = block_time.as_millis(),
            );
        } else {
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                debug!(
                    LogSchema::new(LogEntry::GetBlock),
                    seen_consensus = exclude_size,
                    walked = txn_walked,
                    // before size and non full check
                    result_size = result_size,
                    // before non full check
                    byte_size = total_bytes,
                    block_size = block.len(),
                    return_non_full = return_non_full,
                    result_time_ms = result_time.as_millis(),
                    block_time_ms = block_time.as_millis(),
                )
            );
        }

        if !return_non_full && !full_bytes && (block.len() as u64) < max_txns {
            block.clear();
        }

        counters::mempool_service_transactions(counters::GET_BLOCK_LABEL, block.len());
        counters::MEMPOOL_SERVICE_BYTES_GET_BLOCK.observe(total_bytes as f64);
        for transaction in &block {
            self.log_consensus_pulled_latency(transaction.sender(), transaction.replay_protector());
        }
        block
    }

    /// Periodic core mempool garbage collection.
    /// Removes all expired transactions and clears expired entries in metrics
    /// cache and sequence number cache.
    pub(crate) fn gc(&mut self) {
        let now = aptos_infallible::duration_since_epoch();
        self.transactions.gc_by_system_ttl(now);
    }

    /// Garbage collection based on client-specified expiration time.
    pub(crate) fn gc_by_expiration_time(&mut self, block_time: Duration) {
        self.transactions.gc_by_expiration_time(block_time);
    }

    /// Returns block of transactions and new last_timeline_id. For each transaction, the output includes
    /// the transaction ready time in millis since epoch
    pub(crate) fn read_timeline(
        &self,
        sender_bucket: MempoolSenderBucket,
        timeline_id: &MultiBucketTimelineIndexIds,
        count: usize,
        before: Option<Instant>,
        priority_of_receiver: BroadcastPeerPriority,
    ) -> (Vec<(SignedTransaction, u64)>, MultiBucketTimelineIndexIds) {
        self.transactions.read_timeline(
            sender_bucket,
            timeline_id,
            count,
            before,
            priority_of_receiver,
        )
    }

    /// Read transactions from timeline from `start_id` (exclusive) to `end_id` (inclusive),
    /// along with their ready times in millis since poch
    pub(crate) fn timeline_range(
        &self,
        sender_bucket: MempoolSenderBucket,
        start_end_pairs: HashMap<TimelineIndexIdentifier, (u64, u64)>,
    ) -> Vec<(SignedTransaction, u64)> {
        self.transactions
            .timeline_range(sender_bucket, start_end_pairs)
    }

    pub(crate) fn timeline_range_of_message(
        &self,
        sender_start_end_pairs: HashMap<
            MempoolSenderBucket,
            HashMap<TimelineIndexIdentifier, (u64, u64)>,
        >,
    ) -> Vec<(SignedTransaction, u64)> {
        sender_start_end_pairs
            .iter()
            .flat_map(|(sender_bucket, start_end_pairs)| {
                self.transactions
                    .timeline_range(*sender_bucket, start_end_pairs.clone())
            })
            .collect()
    }

    pub fn gen_snapshot(&self) -> TxnsLog {
        self.transactions.gen_snapshot()
    }

    #[cfg(test)]
    pub fn get_parking_lot_size(&self) -> usize {
        self.transactions.get_parking_lot_size()
    }

    #[cfg(test)]
    pub fn get_transaction_store(&self) -> &TransactionStore {
        &self.transactions
    }

    pub fn get_parking_lot_addresses(&self) -> Vec<(AccountAddress, u64)> {
        self.transactions.get_parking_lot_addresses()
    }
}
